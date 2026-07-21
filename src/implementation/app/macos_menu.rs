mod menu_app;
mod menu_edit;
mod menu_file;
mod menu_file_export;
mod menu_file_items;
mod menu_help;
mod menu_install;
mod menu_state;
mod menu_tab;
mod menu_target;
mod menu_view;

pub use self::menu_install::install;
pub use self::menu_state::{set_document_output_enabled, set_selected_theme};
