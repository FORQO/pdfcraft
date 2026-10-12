//! Custom stamps (Add a stamp ▸ Custom stamps ▸ Create): a PDF page or an image saved in the
//! user's stamp library under a category and a name, then placed like any stamp.

use std::sync::Arc;

use egui::{Align, Layout};
use serde::{Deserialize, Serialize};

use crate::theme::{self, Tokens};
use crate::{Dialog, PdfCraftApp, QuickTool, widgets};

/// Editable wording for a company representative's translation statement.
/// This creates text to be placed into a PDF, not a digital or notarial signature.
#[derive(Clone, Debug, PartialEq)]
pub struct TranslationStatementDraft {
    pub from_language: String,
    pub organization: String,
    pub signer: String,
    pub position: String,
    pub date: String,
}

impl Default for TranslationStatementDraft {
    fn default() -> Self {
        Self {
            from_language: String::new(),
            organization: String::new(),
            signer: String::new(),
            position: "Директор".to_owned(),
            date: String::new(),
        }
    }
}

impl TranslationStatementDraft {
    pub fn text(&self) -> String {
        let mut lines = vec!["ПЕРЕВОД ВЕРЕН".to_owned()];
        if !self.from_language.trim().is_empty() {
            lines.push(format!("Перевод с языка: {}", self.from_language.trim()));
        }
        lines.push("Соответствие перевода представленному документу подтверждаю.".to_owned());
        if !self.organization.trim().is_empty() {
            lines.push(format!("Организация: {}", self.organization.trim()));
        }
        let position = self.position.trim();
        let signer = self.signer.trim();
        lines.push(format!("{position} __________________ / {signer}"));
        if !self.date.trim().is_empty() {
            lines.push(format!("Дата: {}", self.date.trim()));
        }
        lines.join("\n")
    }
}

/// Copy a customisable translation statement, then select the existing PDF text tool.
/// The user places the text on a page and supplies an actual signature separately.
fn translation_statement_section(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    let mut copy = false;
    ui.collapsing(tl!("Translation verification statement"), |ui| {
        let draft = &mut app.translation_statement;
        ui.label(
            egui::RichText::new(tl!("Editable statement for translations; not a notarial certificate or digital signature."))
                .small()
                .color(t.text_muted),
        );
        egui::Grid::new("ru-translation-statement").num_columns(2).spacing([8.0, 8.0]).show(ui, |ui| {
            for (label, value) in [
                (tl!("Source language"), &mut draft.from_language),
                (tl!("Organization"), &mut draft.organization),
                (tl!("Position"), &mut draft.position),
                (tl!("Full name"), &mut draft.signer),
                (tl!("Date"), &mut draft.date),
            ] {
                let caption = ui.label(label);
                ui.add(egui::TextEdit::singleline(value).desired_width(160.0)).labelled_by(caption.id);
                ui.end_row();
            }
        });
        if ui.button(tl!("Prepare statement and select text tool")).clicked() {
            copy = true;
        }
        ui.label(
            egui::RichText::new(tl!("Click a page to place the prepared text; add the director's signature separately.")).small().color(t.text_faint),
        );
    });
    if copy {
        let text = app.translation_statement.text();
        ui.ctx().copy_text(text.clone());
        if let Some((index, _)) = app.active_ids() {
            app.views[index].content.pending_text = Some(text);
        }
        app.quick_tool = QuickTool::AddText;
        app.left = crate::LeftPanel::Tool("edit");
        app.left_open = true;
    }
}

/// Stamp files larger than this aren't kept in the library (it lives in the app's settings).
pub const MAX_STAMP_BYTES: usize = 4 << 20;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CustomStamp {
    pub category: String,
    pub name: String,
    /// The source file's name (its extension tells images from PDFs).
    pub file: String,
    /// The page of a PDF (0-based).
    #[serde(default)]
    pub page: usize,
    #[serde(with = "b64")]
    pub data: Arc<Vec<u8>>,
}

mod b64 {
    use std::sync::Arc;

    use base64::Engine as _;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(v: &Arc<Vec<u8>>, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&base64::engine::general_purpose::STANDARD.encode(v.as_slice()))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Arc<Vec<u8>>, D::Error> {
        let s = String::deserialize(d)?;
        base64::engine::general_purpose::STANDARD.decode(s).map(Arc::new).map_err(serde::de::Error::custom)
    }
}

/// The Create Custom Stamp dialog: the chosen file, its category and name.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StampDraft {
    pub file: String,
    pub data: Arc<Vec<u8>>,
    pub category: String,
    pub name: String,
}

pub(crate) fn create_body(ui: &mut egui::Ui, app: &mut PdfCraftApp, t: &Tokens) -> (bool, bool) {
    let categories: Vec<String> = {
        let mut c: Vec<String> = app.custom_stamps.iter().map(|s| s.category.clone()).collect();
        c.sort();
        c.dedup();
        c
    };
    let d = &mut app.stamp_draft;
    ui.label(egui::RichText::new(tl!("Create Custom Stamp")).font(theme::semibold(18.0)));
    ui.add_space(8.0);
    ui.label(egui::RichText::new(crate::i18n::fmt(tl!("From {file}"), &[("file", &d.file)])).color(t.text_muted));
    ui.add_space(8.0);
    egui::Grid::new("stamp-create").num_columns(2).spacing([10.0, 8.0]).show(ui, |ui| {
        let l = ui.label(tl!("Category:"));
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut d.category).desired_width(200.0).hint_text(tl!("e.g. My stamps"))).labelled_by(l.id);
            if !categories.is_empty() {
                egui::ComboBox::from_id_salt("stamp-categories").selected_text("").width(24.0).show_ui(ui, |ui| {
                    for c in &categories {
                        if ui.selectable_label(d.category == *c, c).clicked() {
                            d.category = c.clone();
                        }
                    }
                });
            }
        });
        ui.end_row();
        let l = ui.label(tl!("Name:"));
        ui.add(egui::TextEdit::singleline(&mut d.name).desired_width(200.0)).labelled_by(l.id);
        ui.end_row();
    });
    ui.add_space(12.0);
    let ok = !d.category.trim().is_empty() && !d.name.trim().is_empty();
    let (mut save, mut cancel) = (false, false);
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        if ui.add_enabled_ui(ok, |ui| widgets::pill_button(ui, tl!("OK"), true)).inner.clicked() {
            save = true;
        }
        if widgets::pill_button(ui, tl!("Cancel"), false).clicked() {
            cancel = true;
        }
    });
    (save, cancel)
}

/// The Custom stamps section of the stamps palette.
pub(crate) fn palette_section(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    widgets::section_title(ui, tl!("Custom"));
    let mut remove = None;
    let mut categories: Vec<String> = app.custom_stamps.iter().map(|s| s.category.clone()).collect();
    categories.sort();
    categories.dedup();
    for c in categories {
        ui.label(egui::RichText::new(&c).small().color(t.text_muted));
        for (i, s) in app.custom_stamps.iter().enumerate().filter(|(_, s)| s.category == c) {
            let active = app.quick_tool == QuickTool::CustomStamp(i);
            let resp = ui.add(egui::Button::selectable(active, s.name.as_str()).min_size(egui::vec2(ui.available_width(), 28.0)));
            if resp.clicked() {
                app.quick_tool = QuickTool::CustomStamp(i);
            }
            resp.context_menu(|ui| {
                if ui.button(tl!("Delete stamp")).clicked() {
                    remove = Some(i);
                    ui.close();
                }
            });
        }
    }
    if let Some(i) = remove {
        app.custom_stamps.remove(i);
        app.quick_tool = QuickTool::Select;
    }
    ui.add_space(4.0);
    if widgets::pill_button(ui, tl!("Create custom stamp…"), false).on_hover_text(tl!("From a PDF page or an image")).clicked() {
        app.pick_stamp_file();
    }
    ui.add_space(8.0);
    translation_statement_section(app, ui, t);
}

impl PdfCraftApp {
    /// Create ▸ choose a PDF or an image for a new custom stamp.
    pub(crate) fn pick_stamp_file(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let read = |app: &mut Self, path: std::path::PathBuf| match std::fs::read(&path) {
                Ok(bytes) => {
                    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                    app.start_custom_stamp(name, bytes);
                }
                Err(e) => app.notify_fmt("Couldn't read {name}: {e}", &[("name", &path.display().to_string()), ("e", &e.to_string())]),
            };
            match self.save_override.clone() {
                Some(p) if [".png", ".jpg", ".pdf"].iter().any(|e| p.ends_with(e)) => read(self, p.into()),
                Some(_) => {}
                None => {
                    let dialog = rfd::AsyncFileDialog::new()
                        .add_filter(tl!("PDF or image"), &["pdf", "png", "jpg", "jpeg", "tif", "tiff", "gif", "bmp", "jp2", "j2k", "jpx"])
                        .set_title(tl!("Select a file for the stamp"));
                    self.ask_one(crate::pickers::Ask::File(dialog), None, read);
                }
            }
        }
        #[cfg(target_arch = "wasm32")]
        self.notify_tr("Custom stamps arrive on the web with file pickers for images");
    }

    /// Open the Create Custom Stamp dialog for a file.
    pub fn start_custom_stamp(&mut self, file: String, bytes: Vec<u8>) {
        if bytes.len() > MAX_STAMP_BYTES {
            self.notify_fmt("{file} is too large for a stamp (at most {n} MB)", &[("file", &file), ("n", &(MAX_STAMP_BYTES >> 20).to_string())]);
            return;
        }
        let name = file.rsplit_once('.').map_or(file.as_str(), |(s, _)| s).to_string();
        let category = self.custom_stamps.last().map(|s| s.category.clone()).unwrap_or_else(|| "My stamps".into());
        self.stamp_draft = crate::stamps_ui::StampDraft { file, data: Arc::new(bytes), category, name };
        self.dialog = Some(Dialog::CreateStamp);
    }

    /// Save the dialog's stamp in the library and choose it.
    pub(crate) fn save_custom_stamp(&mut self) {
        let d = std::mem::take(&mut self.stamp_draft);
        self.custom_stamps.push(CustomStamp { category: d.category.trim().into(), name: d.name.trim().into(), file: d.file, page: 0, data: d.data });
        self.quick_tool = QuickTool::CustomStamp(self.custom_stamps.len() - 1);
    }
}

/// The custom stamps for `persist` (kept small: base64 of each file).
pub(crate) fn encode(stamps: &[CustomStamp]) -> serde_json::Value {
    serde_json::to_value(stamps).unwrap_or_default()
}

/// Read stamps back; malformed entries are dropped.
pub(crate) fn decode(v: &serde_json::Value) -> Vec<CustomStamp> {
    v.as_array()
        .map(|a| a.iter().filter_map(|x| serde_json::from_value::<CustomStamp>(x.clone()).ok()).filter(|s| s.data.len() <= MAX_STAMP_BYTES).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod translation_statement_tests {
    use super::TranslationStatementDraft;

    #[test]
    fn translation_statement_contains_signer_and_date() {
        let d = TranslationStatementDraft {
            from_language: "английского".into(),
            organization: "ООО Пример".into(),
            signer: "И. И. Иванов".into(),
            position: "Директор".into(),
            date: "09.10.2026".into(),
        };
        let text = d.text();
        assert!(text.starts_with("ПЕРЕВОД ВЕРЕН"));
        assert!(text.contains("ООО Пример"));
        assert!(text.contains("Директор __________________ / И. И. Иванов"));
        assert!(text.contains("Дата: 09.10.2026"));
    }

    #[test]
    fn translation_statement_works_without_optional_fields() {
        let text = TranslationStatementDraft::default().text();
        assert!(text.contains("ПЕРЕВОД ВЕРЕН"));
        assert!(text.contains("Директор"));
    }
}
