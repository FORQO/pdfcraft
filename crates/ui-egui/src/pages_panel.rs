//! Direct page assembly in the sidebar: reorder, delete and native PDF drops.
//! Geometry belongs to one DocView and generation, never to a different document.

#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;

use egui::{Color32, CornerRadius, Key, Modifiers, Pos2, Rect, Sense, Stroke, pos2, vec2};
use pdfcraft_engine::Edit;
use pdfcraft_render::DocInfo;

use crate::theme::{self, Tokens};
use crate::{DocView, Mode, PdfCraftApp, RightPanel};

#[derive(Default)]
pub(crate) struct State {
    bounds: Option<Rect>,
    cells: Vec<(usize, Rect)>,
    generation: u64,
    drag: Option<Vec<usize>>,
    keyboard: bool,
}

impl State {
    pub(crate) fn invalidate(&mut self) {
        self.bounds = None;
        self.cells.clear();
        self.drag = None;
        // Keep focus across page moves; Delete releases ownership explicitly.
    }

    fn gap(&self, pointer: Pos2) -> Option<usize> {
        if !self.bounds.is_some_and(|bounds| bounds.contains(pointer)) {
            return None;
        }
        let (index, rect) = self.cells.iter().min_by(|(_, a), (_, b)| a.distance_sq_to_pos(pointer).total_cmp(&b.distance_sq_to_pos(pointer)))?;
        Some(if pointer.y < rect.center().y { *index } else { index.saturating_add(1) })
    }
}

fn showing(app: &PdfCraftApp) -> bool {
    app.right == Some(RightPanel::Pages) && app.mode != Mode::Read && !app.full_screen
}

fn blocked(app: &PdfCraftApp) -> bool {
    app.dialog.is_some() || app.palette_open || app.close_request.is_some() || app.password_prompt.is_some()
}

/// Consume the sidebar's keys before the document canvas sees them. Merely
/// having the panel open must not make Delete remove pages while typing.
pub(crate) fn keys(app: &mut PdfCraftApp, ctx: &egui::Context) {
    let visible = showing(app);
    let modal = blocked(app);
    let active = app.active;
    let focused = ctx.input(|input| input.focused);
    for (index, view) in app.views.iter_mut().enumerate() {
        if !visible || active != Some(index) || !focused || modal {
            view.page_strip.keyboard = false;
            view.page_strip.drag = None;
        }
    }
    let Some((index, id)) = app.active_ids() else { return };
    if !visible || modal || !focused {
        return;
    }
    let view = &mut app.views[index];
    if ctx.input(|input| {
        input.pointer.any_pressed() && input.pointer.interact_pos().is_none_or(|pos| !view.page_strip.bounds.is_some_and(|r| r.contains(pos)))
    }) {
        view.page_strip.keyboard = false;
    }
    if !view.page_strip.keyboard || ctx.egui_wants_keyboard_input() || view.forms.focus.is_some() {
        return;
    }
    if ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape)) {
        view.page_strip.drag = None;
        view.page_strip.keyboard = false;
        view.clear_page_selection(Some(view.current));
        return;
    }
    let delete = ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Delete) || input.consume_key(Modifiers::NONE, Key::Backspace));
    if delete && app.session.get(id).is_some_and(|doc| doc.allows_assembly() && doc.read_only_reason.is_none()) {
        // The engine also rejects removal of the last page, with a visible error.
        let pages = app.views[index].target_pages();
        app.views[index].page_strip.drag = None;
        app.views[index].page_strip.keyboard = false;
        if app.commit_form_typing() {
            app.apply_edit(Edit::DeletePages { pages });
        }
    }
}

/// `to` is in the list with the moving pages removed, unlike a visual gap.
fn move_to_gap(pages: &[usize], gap: usize, count: usize) -> Option<Edit> {
    if pages.is_empty() || gap > count || pages.iter().any(|page| *page >= count) {
        return None;
    }
    let pages: Vec<_> = pages.iter().copied().collect::<std::collections::BTreeSet<_>>().into_iter().collect();
    let to = gap.saturating_sub(pages.iter().filter(|page| **page < gap).count());
    let contiguous = pages.windows(2).all(|pair| matches!(pair, [a, b] if a.checked_add(1) == Some(*b)));
    if contiguous && pages.first() == Some(&to) {
        return None;
    }
    Some(Edit::MovePages { pages, to })
}

/// Draw the strip inside its existing ScrollArea. Returns a page to navigate to.
pub(crate) fn show(ui: &mut egui::Ui, t: &Tokens, info: &DocInfo, view: &mut DocView, editable: bool, modal: bool, generation: u64) -> Option<usize> {
    if view.page_strip.generation != generation {
        view.page_strip.invalidate();
    }
    view.page_strip.generation = generation;
    view.page_strip.bounds = Some(ui.clip_rect());
    view.page_strip.cells.clear();
    if modal || !editable || !ui.input(|input| input.focused) {
        view.page_strip.drag = None;
    }
    let mut nav = None;
    let w = (ui.available_width() - 40.0).clamp(32.0, 150.0);
    for (index, page) in info.pages.iter().enumerate() {
        let row = ui.vertical_centered(|ui| {
            let height = (w * page.height / page.width.max(1.0)).clamp(24.0, 600.0);
            let sense = if editable && !modal { Sense::click_and_drag() } else { Sense::click() };
            let (rect, response) = ui.allocate_exact_size(vec2(w + 16.0, height + 16.0), sense);
            if response.drag_started_by(egui::PointerButton::Primary) {
                if !view.selected.contains(&index) {
                    view.click_page(index, Modifiers::NONE, false);
                }
                view.page_strip.drag = Some(view.selected.iter().copied().collect());
                view.page_strip.keyboard = true;
            }
            let picked = view.selected.contains(&index);
            let current = index == view.current;
            let label = crate::i18n::fmt(tl!("Page {label}"), &[("label", &page.label)]);
            response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, picked, &label));
            if current || picked || response.hovered() {
                ui.painter().rect_filled(rect, CornerRadius::same(8), if current || picked { t.accent_soft } else { t.hover });
            }
            if picked {
                ui.painter().rect_stroke(rect, CornerRadius::same(8), Stroke::new(1.5, t.accent), egui::StrokeKind::Inside);
            }
            let page_rect = rect.shrink(8.0);
            ui.painter().rect_filled(page_rect, CornerRadius::ZERO, Color32::WHITE);
            if let Some(texture) = view.thumb(index) {
                ui.painter().image(texture.id(), page_rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            }
            ui.painter().rect_stroke(
                page_rect,
                CornerRadius::ZERO,
                Stroke::new(if current { 2.0 } else { 1.0 }, if current { t.accent } else { t.border }),
                egui::StrokeKind::Outside,
            );
            ui.label(egui::RichText::new(&page.label).font(theme::medium(12.0)).color(if current || picked { t.accent_text } else { t.text_muted }));
            if response.clicked() && !modal {
                view.page_strip.keyboard = true;
                let modifiers = ui.input(|input| input.modifiers);
                if modifiers.shift || modifiers.command {
                    view.click_page(index, modifiers, true);
                } else {
                    view.clear_page_selection(Some(index));
                    nav = Some(index);
                }
            }
        });
        view.page_strip.cells.push((index, row.response.rect));
        ui.add_space(4.0);
    }
    let pointer = ui.input(|input| input.pointer.hover_pos());
    let gap = pointer.and_then(|pos| view.page_strip.gap(pos));
    let external = ui.input(|input| !input.raw.hovered_files.is_empty());
    let dragging = editable && !modal && (external || view.page_strip.drag.is_some());
    if dragging && let (Some(pos), Some(gap), Some(bounds)) = (pointer, gap, view.page_strip.bounds) {
        let y = view
            .page_strip
            .cells
            .iter()
            .find(|(index, _)| *index == gap)
            .map(|(_, rect)| rect.top() - 2.0)
            .or_else(|| view.page_strip.cells.last().map(|(_, rect)| rect.bottom() + 2.0));
        if let Some(y) = y {
            ui.painter().line_segment([pos2(bounds.left() + 10.0, y), pos2(bounds.right() - 10.0, y)], Stroke::new(3.0, t.accent));
        }
        ui.ctx().set_cursor_icon(if external { egui::CursorIcon::Copy } else { egui::CursorIcon::Grabbing });
        // Auto-scroll keeps distant gaps reachable without dropping the selection.
        let delta = if pos.y < bounds.top() + 30.0 {
            10.0
        } else if pos.y > bounds.bottom() - 30.0 {
            -10.0
        } else {
            0.0
        };
        if delta != 0.0 {
            ui.scroll_with_delta_animation(vec2(0.0, delta), egui::style::ScrollAnimation::none());
            ui.ctx().request_repaint();
        }
    }
    if ui.input(|input| input.pointer.button_released(egui::PointerButton::Primary)) {
        if let Some(pages) = view.page_strip.drag.take()
            && editable
            && !modal
            && let Some(gap) = gap
            && let Some(edit) = move_to_gap(&pages, gap, info.pages.len())
        {
            // Selection is remapped only after apply_edit succeeds.
            view.pending_edit = Some(edit);
        }
    } else if !ui.input(|input| input.pointer.any_down()) {
        view.page_strip.drag = None;
    }
    nav
}

/// Route native file drops to the sidebar, leaving drops elsewhere to Open/grid.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn take_drop(app: &mut PdfCraftApp, dropped: Vec<egui::DroppedFileHandle>, ctx: &egui::Context) -> Vec<egui::DroppedFileHandle> {
    if dropped.is_empty() || !showing(app) {
        return dropped;
    }
    let Some((index, id)) = app.active_ids() else { return dropped };
    let gap = ctx.input(|input| input.pointer.hover_pos()).and_then(|pos| app.views[index].page_strip.gap(pos));
    let Some(gap) = gap else { return dropped };
    let current = app.session.get(id).is_some_and(|doc| {
        doc.edit_generation() == app.views[index].page_strip.generation && doc.allows_assembly() && doc.read_only_reason.is_none()
    });
    if blocked(app) || !current {
        app.notify_tr("The document changed while you were choosing a file, so nothing was changed.");
        return Vec::new();
    }
    // Parsing and applying both run behind the same panic guard as native pickers.
    let result = pdfcraft_engine::guard(|| insert_drop(app, dropped, gap));
    match result {
        Ok(Ok(())) => {}
        Ok(Err(error)) => app.notify_fmt("Couldn't insert pages: {e}", &[("e", &error)]),
        Err(error) => app.notify_fmt("That didn't work: an internal error stopped it ({m}).", &[("m", error.as_str())]),
    }
    Vec::new()
}

#[cfg(not(target_arch = "wasm32"))]
fn insert_drop(app: &mut PdfCraftApp, dropped: Vec<egui::DroppedFileHandle>, gap: usize) -> Result<(), String> {
    use std::io::Read as _;
    const MAX_BYTES: usize = 256 * 1024 * 1024;
    if dropped.len() > pdfcraft_engine::MAX_CREATE_FILES {
        return Err(crate::i18n::t("Too many files in one drop").into());
    }
    let mut edits = Vec::new();
    let mut at = gap;
    let mut total = 0usize;
    for file in dropped {
        if !file.path().extension().is_some_and(|ext| ext.eq_ignore_ascii_case("pdf")) {
            return Err(crate::i18n::t("Drop PDF files on the Pages panel").into());
        }
        let name = file.path().file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_else(|| "dropped.pdf".into());
        let bytes = if file.path().is_absolute() {
            let mut bytes = Vec::new();
            std::fs::File::open(file.path())
                .map_err(|error| error.to_string())?
                .take((MAX_BYTES.saturating_sub(total) + 1) as u64)
                .read_to_end(&mut bytes)
                .map_err(|error| error.to_string())?;
            bytes
        } else {
            file.bytes()?
        };
        total = total.saturating_add(bytes.len());
        if total > MAX_BYTES {
            return Err(crate::i18n::t("PDF files in one drop exceed 256 MiB").into());
        }
        let bytes = Arc::new(bytes);
        let count = app.session.page_count_of(&name, &bytes).map_err(|error| error.to_string())?;
        edits.push(Edit::InsertPagesFrom { name, bytes, pages: None, at });
        at = at.checked_add(count).ok_or("Too many pages")?;
    }
    // Validate every input first, then one undoable transaction. A malformed
    // second PDF must not leave the first PDF inserted without telling the user.
    if !edits.is_empty()
        && app.commit_form_typing()
        && app.apply_edit(Edit::Batch { label: "Insert pages".into(), edits })
        && let Some(view) = app.active.and_then(|index| app.views.get_mut(index))
    {
        view.select_pages(&(gap..at).collect::<Vec<_>>());
        view.page_strip.keyboard = true;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moving_to_visual_gaps_preserves_the_document_order() {
        assert_eq!(move_to_gap(&[3, 1, 1], 5, 6), Some(Edit::MovePages { pages: vec![1, 3], to: 3 }));
        assert_eq!(move_to_gap(&[3, 4], 0, 6), Some(Edit::MovePages { pages: vec![3, 4], to: 0 }));
        assert_eq!(move_to_gap(&[0, 1], 6, 6), Some(Edit::MovePages { pages: vec![0, 1], to: 4 }));
    }

    #[test]
    fn empty_invalid_and_noop_moves_do_not_create_history() {
        assert_eq!(move_to_gap(&[], 0, 5), None);
        assert_eq!(move_to_gap(&[5], 0, 5), None);
        assert_eq!(move_to_gap(&[0], 6, 5), None);
        for gap in 1..=3 {
            assert_eq!(move_to_gap(&[1, 2], gap, 5), None);
        }
    }

    #[test]
    fn vertical_gaps_include_both_ends_but_not_outside_the_panel() {
        let mut state = State { bounds: Some(Rect::from_min_max(pos2(10.0, 0.0), pos2(200.0, 300.0))), ..Default::default() };
        for index in 0..3 {
            state.cells.push((index, Rect::from_min_size(pos2(20.0, index as f32 * 100.0), vec2(150.0, 90.0))));
        }
        assert_eq!(state.gap(pos2(100.0, 1.0)), Some(0));
        assert_eq!(state.gap(pos2(100.0, 97.0)), Some(1));
        assert_eq!(state.gap(pos2(100.0, 298.0)), Some(3));
        assert_eq!(state.gap(pos2(300.0, 100.0)), None);
        state.invalidate();
        assert_eq!(state.gap(pos2(100.0, 97.0)), None);
    }
}
