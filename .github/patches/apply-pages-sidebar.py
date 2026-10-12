"""Temporary integration helper; touches only six named UI source/test files."""
from pathlib import Path

root = Path("crates/ui-egui")

def replace_once(path: Path, old: str, new: str) -> None:
    text = path.read_text(encoding="utf-8")
    if text.count(old) != 1:
        raise RuntimeError(f"Source changed at {path}: expected exactly one integration anchor")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")

replace_once(root / "src/canvas.rs", "    pub grid_gap: Option<usize>,\n", "    pub grid_gap: Option<usize>,\n    /// Transient sidebar gestures and hit testing; never persisted.\n    pub(crate) page_strip: crate::pages_panel::State,\n")
replace_once(root / "src/canvas.rs", "            grid_gap: None,\n", "            grid_gap: None,\n            page_strip: Default::default(),\n")
replace_once(root / "src/canvas.rs", "    pub fn document_changed(&mut self, info: &DocInfo) {\n", "    pub fn document_changed(&mut self, info: &DocInfo) {\n        self.page_strip.invalidate();\n")
replace_once(root / "src/lib.rs", "mod panels;\n", "mod panels;\nmod pages_panel;\n")
replace_once(root / "src/lib.rs", "        #[cfg(not(target_arch = \"wasm32\"))]\n        let dropped = self.drop_on_grid(dropped, ctx);", "        #[cfg(not(target_arch = \"wasm32\"))]\n        let dropped = pages_panel::take_drop(self, dropped, ctx);\n        #[cfg(not(target_arch = \"wasm32\"))]\n        let dropped = self.drop_on_grid(dropped, ctx);")
replace_once(root / "src/lib.rs", "        self.shortcuts(ctx);\n", "        pages_panel::keys(self, ctx);\n        self.shortcuts(ctx);\n")
replace_once(root / "src/panels.rs", "let modal = app.dialog.is_some() || app.palette_open;", "let modal = app.dialog.is_some() || app.palette_open || app.close_request.is_some() || app.password_prompt.is_some();")
old_count = '''                    if panel == RightPanel::Pages && view.selected.len() > 1 {
                        let n = view.selected.len().to_string();
                        let picked = crate::i18n::fmt(tl!("{n} pages selected"), &[("n", &n)]);
                        ui.label(egui::RichText::new(picked).font(theme::medium(12.0)).color(t.accent_text));
                    }
'''
replace_once(root / "src/panels.rs", old_count, "")
new_count = '''                // Keep the selection count below the title: Russian text must not
                // overlap the panel's close button at the minimum width.
                if panel == RightPanel::Pages && view.selected.len() > 1 {
                    let n = view.selected.len().to_string();
                    let picked = crate::i18n::fmt(tl!("{n} pages selected"), &[("n", &n)]);
                    ui.label(egui::RichText::new(picked).font(theme::medium(12.0)).color(t.accent_text));
                }
'''
replace_once(root / "src/panels.rs", '                ui.add_space(6.0);\n                let search_id = egui::Id::new(("bookmark_search", id.0));', new_count + '                ui.add_space(6.0);\n                let search_id = egui::Id::new(("bookmark_search", id.0));')
replace_once(root / "src/panels.rs", "                    RightPanel::Pages => pages(ui, &t, info, view, modal, &mut nav),", '''                    RightPanel::Pages => {
                        if let Some(page) = crate::pages_panel::show(ui, &t, info, view, bm_editable, modal, doc.edit_generation()) {
                            nav = Some(Nav::Page(page));
                        }
                    }''')
path = root / "src/panels.rs"
text = path.read_text(encoding="utf-8")
start = text.index("/// The page thumbnails. A click goes to the page;")
end = text.index("/// The Fields panel:", start)
if "fn pages(" not in text[start:end] or "fn fields(" in text[start:end]:
    raise RuntimeError("Unexpected old sidebar function bounds")
path.write_text(text[:start] + text[end:], encoding="utf-8")
for src, dest in [("pages-sidebar-module.rs", "src/pages_panel.rs"), ("pages-sidebar-tests.rs", "tests/pages_panel.rs")]:
    path = root / dest
    if path.exists():
        raise RuntimeError(f"New module already exists: {path}")
    path.write_bytes((Path(".github/patches") / src).read_bytes())
path = root / "src/i18n/ru.tsv"
text = path.read_text(encoding="utf-8")
rows = [
    ("Couldn't insert pages: {e}", "Не удалось вставить страницы: {e}"),
    ("Too many files in one drop", "Слишком много файлов для одной вставки"),
    ("Drop PDF files on the Pages panel", "Перетащите файлы PDF в панель «Страницы»"),
    ("PDF files in one drop exceed 256 MiB", "Общий размер файлов PDF для одной вставки превышает 256 МиБ"),
]
for source, translation in rows:
    if f"\t{source}\t" in text:
        raise RuntimeError(f"Translation already exists: {source}")
text += "\n# Direct page assembly in the sidebar\n" + "".join(f"\t{s}\t{t}\n" for s, t in rows)
path.write_text(text, encoding="utf-8")
