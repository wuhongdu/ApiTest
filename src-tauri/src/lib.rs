mod commands;
mod db;
mod http;
mod jmeter;
mod loadtest;
mod models;
mod openapi;
mod postman;

use db::DbState;
use commands::SendCancelState;
use tauri::Manager;

#[cfg(test)]
mod feature_tests;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_data = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data dir");
            std::fs::create_dir_all(&app_data).expect("failed to create app data dir");

            let db_path = app_data.join("apitest.db");
            let conn = db::open_and_migrate(&db_path).expect("failed to open sqlite database");
            app.manage(DbState::new(conn, db_path));
            app.manage(SendCancelState::new());

            // Ensure taskbar / Alt-Tab use the app icon (esp. with decorations: false).
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_icon(tauri::include_image!("icons/128x128.png"));
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::ping,
            commands::db_version,
            commands::db_path,
            commands::list_collections,
            commands::create_collection,
            commands::rename_collection,
            commands::update_collection,
            commands::delete_collection,
            commands::get_sidebar_tree,
            commands::get_request,
            commands::create_request,
            commands::save_request,
            commands::delete_request,
            commands::list_environments,
            commands::set_active_environment,
            commands::list_env_vars,
            commands::upsert_env_var,
            commands::delete_env_var,
            commands::create_environment,
            commands::send_request,
            commands::cancel_request,
            commands::preview_substituted,
            commands::list_history,
            commands::get_history,
            commands::clear_history,
            commands::delete_history,
            commands::export_collection,
            commands::export_jmeter,
            commands::import_collection,
            commands::run_load_test_cmd,
            commands::list_workspaces,
            commands::create_workspace,
            commands::set_active_workspace,
            commands::get_active_workspace_id,
            commands::delete_workspace,
            commands::get_theme,
            commands::set_theme,
        ])
        .run(tauri::generate_context!())
        .expect("error while running ApiTest");
}
