"""Refine focus ownership and extend sidebar regression coverage before commit."""
from pathlib import Path

def replace_once(path, old, new):
    path = Path(path)
    text = path.read_text(encoding="utf-8")
    if text.count(old) != 1:
        raise RuntimeError(f"Changed refinement anchor: {path}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")

module = "crates/ui-egui/src/pages_panel.rs"
replace_once(module, "egui::WidgetInfo::selected(egui::WidgetType::Button, true, picked, label)", "egui::WidgetInfo::selected(egui::WidgetType::Button, true, picked, &label)")
replace_once(module, "        self.drag = None;\n        self.keyboard = false;\n", "        self.drag = None;\n        // Keep focus across page moves; Delete releases ownership explicitly.\n")
replace_once(module, "        app.views[index].page_strip.drag = None;\n        if app.commit_form_typing() {", "        app.views[index].page_strip.drag = None;\n        app.views[index].page_strip.keyboard = false;\n        if app.commit_form_typing() {")
replace_once(module, "        view.select_pages(&(gap..at).collect::<Vec<_>>());", "        view.select_pages(&(gap..at).collect::<Vec<_>>());\n        view.page_strip.keyboard = true;")

path = Path("crates/ui-egui/src/i18n/zh-hans.tsv")
text = path.read_text(encoding="utf-8")
rows = [
    ("Couldn't insert pages: {e}", "无法插入页面：{e}"),
    ("Too many files in one drop", "一次拖放的文件过多"),
    ("Drop PDF files on the Pages panel", "将 PDF 文件拖放到“页面”面板"),
    ("PDF files in one drop exceed 256 MiB", "一次拖放的 PDF 文件总大小超过 256 MiB"),
]
for source, translation in rows:
    if f"\t{source}\t" in text:
        raise RuntimeError(f"Translation already exists: {source}")
path.write_text(text + "\n# Direct page assembly in the sidebar\n" + "".join(f"\t{s}\t{t}\n" for s, t in rows), encoding="utf-8")

replace_once(".github/workflows/ru-macos.yml", "      - name: Test PdfCraft RU UI library\n        run: cargo test --locked -p pdfcraft-ui-egui --lib\n", "      - name: Test PdfCraft RU UI library\n        run: cargo test --locked -p pdfcraft-ui-egui --lib\n\n      - name: Test Pages sidebar interactions\n        run: cargo test --locked -p pdfcraft-ui-egui --test pages_panel\n")

path = Path("crates/ui-egui/tests/pages_panel.rs")
text = path.read_text(encoding="utf-8")
text += '''
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
'''
path.write_text(text, encoding="utf-8")
