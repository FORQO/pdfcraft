//! Synthetic UI regressions for drag/drop and keyboard page assembly in the sidebar.

use std::path::Path;
use std::sync::Arc;

use egui::{Key, Modifiers, Pos2, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use pdfcraft_render::{PageRenderer, RenderRequest, RequestKind};
use pdfcraft_ui_egui::{Dialog, PdfCraftApp};

fn fixture(n: usize) -> Vec<u8> {
    let mut objs: Vec<String> = vec!["<< /Type /Catalog /Pages 2 0 R >>".into()];
    let kids: Vec<String> = (0..n).map(|i| format!("{} 0 R", 4 + 2 * i)).collect();
    objs.push(format!("<< /Type /Pages /Kids [{}] /Count {n} /MediaBox [0 0 200 300] >>", kids.join(" ")));
    objs.push("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".into());
    for i in 0..n {
        objs.push(format!("<< /Type /Page /Parent 2 0 R /Contents {} 0 R /Resources << /Font << /F1 3 0 R >> >> >>", 5 + 2 * i));
        let body = format!("BT /F1 24 Tf 20 150 Td (Page {}) Tj ET", i + 1);
        objs.push(format!("<< /Length {} >>\nstream\n{body}\nendstream", body.len()));
    }
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, o) in objs.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n{o}\nendobj\n", i + 1).as_bytes());
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).as_bytes());
    for o in offsets {
        out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objs.len() + 1).as_bytes());
    out
}

fn harness(count: usize) -> Harness<'static, PdfCraftApp> {
    let mut harness = Harness::builder().with_size(vec2(1400.0, 1900.0)).build_eframe(move |_| {
        let mut app = PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        app.open_bytes("source.pdf", None, fixture(count)).unwrap();
        app.set_option("panel", "pages").unwrap();
        app
    });
    harness.run_steps(5);
    harness
}

fn texts(app: &PdfCraftApp) -> Vec<String> {
    let doc = app.session.get(app.views[0].id).unwrap();
    let mut renderer = PageRenderer::new(doc.bytes.clone(), Default::default());
    (0..renderer.page_count())
        .map(|page| {
            renderer
                .render(RenderRequest { page, kind: RequestKind::Text, scale: 1.0, ..Default::default() })
                .text
                .map(|text| text.plain_text().trim().to_string())
                .unwrap_or_default()
        })
        .collect()
}

fn drag(h: &mut Harness<'static, PdfCraftApp>, from: Pos2, to: Pos2) {
    h.hover_at(from);
    h.run_steps(1);
    h.drag_at(from);
    h.run_steps(1);
    for step in 1..=5 {
        h.hover_at(from + (to - from) * (step as f32 / 5.0));
        h.run_steps(1);
    }
    h.drop_at(to);
    h.run_steps(4);
}

#[test]
fn noncontiguous_pages_drag_as_one_block_to_the_end_and_undo() {
    let mut h = harness(5);
    h.get_by_label("Page 3").click_modifiers(Modifiers::COMMAND);
    h.run_steps(4);
    assert_eq!(h.state().views[0].target_pages(), [0, 2]);
    let from = h.get_by_label("Page 3").rect().center();
    let last = h.get_by_label("Page 5").rect();
    drag(&mut h, from, pos2(last.center().x, last.bottom() - 5.0));
    assert_eq!(texts(h.state()), ["Page 2", "Page 4", "Page 5", "Page 1", "Page 3"]);
    assert_eq!(h.state().views[0].target_pages(), [3, 4]);
    h.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    h.run_steps(4);
    assert_eq!(texts(h.state()), ["Page 1", "Page 2", "Page 3", "Page 4", "Page 5"]);
}

#[test]
fn contiguous_pages_can_move_before_the_first_page() {
    let mut h = harness(5);
    h.get_by_label("Page 3").click();
    h.run_steps(3);
    h.get_by_label("Page 4").click_modifiers(Modifiers::SHIFT);
    h.run_steps(4);
    let from = h.get_by_label("Page 3").rect().center();
    let first = h.get_by_label("Page 1").rect();
    drag(&mut h, from, pos2(first.center().x, first.top() + 3.0));
    assert_eq!(texts(h.state()), ["Page 3", "Page 4", "Page 1", "Page 2", "Page 5"]);
}

#[test]
fn releasing_pages_outside_the_sidebar_does_not_edit_the_document() {
    let mut h = harness(4);
    let original = texts(h.state());
    let from = h.get_by_label("Page 2").rect().center();
    drag(&mut h, from, pos2(650.0, 650.0));
    assert_eq!(texts(h.state()), original);
    assert!(!h.state().session.get(h.state().views[0].id).unwrap().dirty);
}

#[test]
fn escape_cancels_a_page_drag_without_modifying_the_document() {
    let mut h = harness(4);
    let from = h.get_by_label("Page 2").rect().center();
    let to = h.get_by_label("Page 4").rect().center();
    h.drag_at(from);
    h.run_steps(1);
    h.hover_at(to);
    h.run_steps(2);
    h.key_press(Key::Escape);
    h.run_steps(2);
    h.drop_at(to);
    h.run_steps(3);
    assert_eq!(texts(h.state()), ["Page 1", "Page 2", "Page 3", "Page 4"]);
}

#[test]
fn delete_removes_selected_pages_and_undo_redo_work() {
    let mut h = harness(5);
    h.get_by_label("Page 3").click_modifiers(Modifiers::COMMAND);
    h.run_steps(3);
    h.key_press(Key::Delete);
    h.run_steps(3);
    assert_eq!(texts(h.state()), ["Page 2", "Page 4", "Page 5"]);
    h.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    h.run_steps(3);
    assert_eq!(texts(h.state()), ["Page 1", "Page 2", "Page 3", "Page 4", "Page 5"]);
    h.state_mut().redo();
    h.run_steps(3);
    assert_eq!(texts(h.state()), ["Page 2", "Page 4", "Page 5"]);
}

#[test]
fn mac_delete_is_backspace_and_can_remove_a_single_clicked_page() {
    let mut h = harness(3);
    h.get_by_label("Page 2").click();
    h.run_steps(3);
    h.key_press(Key::Backspace);
    h.run_steps(3);
    assert_eq!(texts(h.state()), ["Page 1", "Page 3"]);
    // Holding/repeating Delete must not delete an unselected next page.
    h.key_press(Key::Backspace);
    h.run_steps(3);
    assert_eq!(texts(h.state()), ["Page 1", "Page 3"]);
}

#[test]
fn delete_leaves_the_document_alone_after_clicking_the_canvas() {
    let mut h = harness(4);
    h.get_by_label("Page 2").click_modifiers(Modifiers::COMMAND);
    h.run_steps(3);
    h.hover_at(pos2(650.0, 650.0));
    h.run_steps(2);
    h.drag_at(pos2(650.0, 650.0));
    h.drop_at(pos2(650.0, 650.0));
    h.run_steps(3);
    h.key_press(Key::Delete);
    h.run_steps(3);
    assert_eq!(texts(h.state()).len(), 4);
}

#[test]
fn modal_dialog_and_last_page_are_protected() {
    let mut h = harness(1);
    h.get_by_label("Page 1").click();
    h.run_steps(3);
    h.key_press(Key::Delete);
    h.run_steps(3);
    assert_eq!(texts(h.state()), ["Page 1"]);
    h.state_mut().dialog = Some(Dialog::About);
    h.run_steps(3);
    h.key_press(Key::Delete);
    h.run_steps(3);
    assert_eq!(texts(h.state()), ["Page 1"]);
}

#[derive(Debug)]
struct Dropped {
    name: String,
    bytes: Vec<u8>,
}
impl egui::DroppedFile for Dropped {
    fn path(&self) -> &Path {
        Path::new(&self.name)
    }
    fn bytes(&self) -> Result<Vec<u8>, String> {
        Ok(self.bytes.clone())
    }
}

fn drop_files(h: &mut Harness<'static, PdfCraftApp>, files: Vec<(&str, Vec<u8>)>) {
    for (name, bytes) in files {
        h.input_mut().dropped_files.push(Arc::new(Dropped { name: name.into(), bytes }));
    }
    h.run_steps(4);
}

#[test]
fn a_pdf_dropped_between_thumbnails_is_inserted_not_opened() {
    let mut h = harness(3);
    let page = h.get_by_label("Page 2").rect();
    h.hover_at(pos2(page.center().x, page.top() + 2.0));
    h.run_steps(2);
    drop_files(&mut h, vec![("extra.PDF", fixture(2))]);
    assert_eq!(h.state().views.len(), 1);
    assert_eq!(texts(h.state()), ["Page 1", "Page 1", "Page 2", "Page 2", "Page 3"]);
    assert_eq!(h.state().views[0].target_pages(), [1, 2]);
    h.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    h.run_steps(3);
    assert_eq!(texts(h.state()), ["Page 1", "Page 2", "Page 3"]);
}

#[test]
fn multiple_pdf_files_insert_in_order_as_one_undoable_operation() {
    let mut h = harness(3);
    let page = h.get_by_label("Page 1").rect();
    h.hover_at(pos2(page.center().x, page.top() + 3.0));
    h.run_steps(2);
    drop_files(&mut h, vec![("first.pdf", fixture(1)), ("second.pdf", fixture(2))]);
    assert_eq!(texts(h.state()), ["Page 1", "Page 1", "Page 2", "Page 1", "Page 2", "Page 3"]);
    h.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    h.run_steps(3);
    assert_eq!(texts(h.state()), ["Page 1", "Page 2", "Page 3"]);
}

#[test]
fn corrupt_pdf_in_a_multi_file_drop_leaves_every_page_unchanged() {
    let mut h = harness(3);
    let page = h.get_by_label("Page 2").rect();
    h.hover_at(page.center());
    h.run_steps(2);
    drop_files(&mut h, vec![("good.pdf", fixture(1)), ("bad.pdf", b"not a PDF".to_vec())]);
    assert_eq!(texts(h.state()), ["Page 1", "Page 2", "Page 3"]);
    assert_eq!(h.state().views.len(), 1);
    assert!(!h.state().session.get(h.state().views[0].id).unwrap().dirty);
}

#[test]
fn a_drop_on_the_document_not_the_sidebar_still_opens_a_new_tab() {
    let mut h = harness(3);
    h.hover_at(pos2(650.0, 650.0));
    h.run_steps(2);
    drop_files(&mut h, vec![("other.pdf", fixture(1))]);
    assert_eq!(h.state().views.len(), 2);
    assert_eq!(texts(h.state()), ["Page 1", "Page 2", "Page 3"]);
}

#[test]
fn native_file_path_drop_can_append_after_the_last_page() {
    let path = std::env::temp_dir().join(format!("veyra-pages-drop-{}.pdf", std::process::id()));
    std::fs::write(&path, fixture(1)).unwrap();
    let mut h = harness(2);
    let page = h.get_by_label("Page 2").rect();
    h.hover_at(pos2(page.center().x, page.bottom() - 2.0));
    h.run_steps(2);
    drop_files(&mut h, vec![(&path.to_string_lossy(), Vec::new())]);
    std::fs::remove_file(path).unwrap();
    assert_eq!(h.state().views.len(), 1);
    assert_eq!(texts(h.state()), ["Page 1", "Page 2", "Page 1"]);
}

#[test]
fn backspace_in_the_find_field_does_not_delete_selected_pages() {
    let mut h = harness(4);
    h.get_by_label("Page 2").click_modifiers(Modifiers::COMMAND);
    h.run_steps(3);
    h.state_mut().views[0].open_find();
    h.run_steps(4);
    assert!(h.ctx.egui_wants_keyboard_input());
    h.key_press(Key::Backspace);
    h.run_steps(3);
    assert_eq!(texts(h.state()), ["Page 1", "Page 2", "Page 3", "Page 4"]);
}

#[test]
fn non_pdf_drop_on_sidebar_is_rejected_without_opening_or_editing() {
    let mut h = harness(3);
    h.hover_at(h.get_by_label("Page 2").rect().center());
    h.run_steps(2);
    drop_files(&mut h, vec![("document.txt", b"not a PDF".to_vec())]);
    assert_eq!(texts(h.state()), ["Page 1", "Page 2", "Page 3"]);
    assert_eq!(h.state().views.len(), 1);
    assert!(!h.state().session.get(h.state().views[0].id).unwrap().dirty);
}

#[test]
fn hidden_panel_does_not_keep_ownership_of_delete() {
    let mut h = harness(3);
    h.get_by_label("Page 2").click_modifiers(Modifiers::COMMAND);
    h.run_steps(3);
    h.state_mut().set_option("panel", "none").unwrap();
    h.run_steps(3);
    h.key_press(Key::Delete);
    h.run_steps(3);
    assert_eq!(texts(h.state()), ["Page 1", "Page 2", "Page 3"]);
}

#[test]
fn russian_selection_count_is_below_the_close_button() {
    let mut h = harness(4);
    h.state_mut().set_option("language", "ru").unwrap();
    h.state_mut().views[0].select_pages(&[0, 2]);
    h.run_steps(4);
    let count = h.get_by_label_contains("Выбрано 2").rect();
    let closes: Vec<_> = h.get_all_by_label("Закрыть").map(|node| node.rect()).collect();
    assert!(!closes.is_empty());
    assert!(closes.iter().all(|close| !close.intersects(count)), "selection count must not overlap Close");
}

#[test]
fn moved_pages_remain_the_delete_target_without_an_extra_click() {
    let mut h = harness(4);
    h.get_by_label("Page 2").click();
    h.run_steps(3);
    let from = h.get_by_label("Page 2").rect().center();
    let last = h.get_by_label("Page 4").rect();
    drag(&mut h, from, pos2(last.center().x, last.bottom() - 5.0));
    assert_eq!(texts(h.state()), ["Page 1", "Page 3", "Page 4", "Page 2"]);
    h.key_press(Key::Delete);
    h.run_steps(3);
    assert_eq!(texts(h.state()), ["Page 1", "Page 3", "Page 4"]);
}

#[test]
fn inserted_pages_are_the_delete_target_without_an_extra_click() {
    let mut h = harness(3);
    let page = h.get_by_label("Page 2").rect();
    h.hover_at(pos2(page.center().x, page.top() + 3.0));
    h.run_steps(2);
    drop_files(&mut h, vec![("extra.pdf", fixture(2))]);
    assert_eq!(h.state().views[0].target_pages(), [1, 2]);
    h.key_press(Key::Backspace);
    h.run_steps(3);
    assert_eq!(texts(h.state()), ["Page 1", "Page 2", "Page 3"]);
}

#[test]
fn dragging_at_the_viewport_edge_reaches_offscreen_pages() {
    let mut h = harness(10);
    h.set_size(vec2(1400.0, 750.0));
    h.run_steps(5);
    let from = h.get_by_label("Page 1").rect().center();
    let to = pos2(from.x, 730.0);
    h.hover_at(from);
    h.run_steps(1);
    h.drag_at(from);
    h.run_steps(1);
    h.hover_at(to);
    h.run_steps(100);
    h.drop_at(to);
    h.run_steps(4);
    let after = texts(h.state());
    assert_eq!(after.first().map(String::as_str), Some("Page 2"));
    assert!(after.iter().position(|text| text == "Page 1").unwrap() >= 3, "must scroll beyond the first two visible pages");
}
