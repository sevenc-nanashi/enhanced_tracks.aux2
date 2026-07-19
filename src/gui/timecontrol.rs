#![allow(clippy::too_many_arguments)]
use super::*;
use anyhow::Context;
use aviutl2_eframe::egui;

mod drawing;
mod editor;
mod interactions;
mod presets;
mod target;
mod types;

pub use types::*;

// TODO: ショートカットキーを変更できるようにする
static COPY_SHORTCUT: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::C);
static PASTE_SHORTCUT: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::V);
static REVERSE_SHORTCUT: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::R);
static ADD_POINT_SHORTCUT: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::A);
static REMOVE_POINT_SHORTCUT: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::D);
static SEPARATE_HANDLES_SHORTCUT: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::S);
static BEZIER_SEGMENT_MODE_MENU_SHORTCUT: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::B);
static ELASTIC_SEGMENT_MODE_MENU_SHORTCUT: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::E);
static BOUNCE_SEGMENT_MODE_MENU_SHORTCUT: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::O);
