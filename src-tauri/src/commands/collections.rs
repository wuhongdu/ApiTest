use crate::db::DbState;
use crate::models::{
    Collection, CreateCollectionInput, RenameCollectionInput, TreeNode, UpdateCollectionInput,
};
use rusqlite::params;
use tauri::State;

fn active_workspace_id(conn: &rusqlite::Connection) -> Result<i64, String> {
    conn.query_row(
        "SELECT id FROM workspaces WHERE is_active = 1 LIMIT 1",
        [],
        |row| row.get(0),
    )
    .or_else(|_| {
        conn.query_row("SELECT id FROM workspaces ORDER BY id LIMIT 1", [], |row| {
            row.get(0)
        })
    })
    .map_err(|e| e.to_string())
}

fn map_collection(row: &rusqlite::Row<'_>) -> rusqlite::Result<Collection> {
    Ok(Collection {
        id: row.get(0)?,
        parent_id: row.get(1)?,
        name: row.get(2)?,
        sort_order: row.get(3)?,
        workspace_id: row.get(4)?,
        base_url: row.get::<_, Option<String>>(5)?.unwrap_or_default(),
    })
}

fn fetch_collections(
    conn: &rusqlite::Connection,
    workspace_id: i64,
) -> Result<Vec<Collection>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, parent_id, name, sort_order, workspace_id, base_url
             FROM collections
             WHERE workspace_id = ?1
             ORDER BY sort_order, id",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params![workspace_id], map_collection)
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

fn get_collection_by_id(conn: &rusqlite::Connection, id: i64) -> Result<Collection, String> {
    conn.query_row(
        "SELECT id, parent_id, name, sort_order, workspace_id, base_url
         FROM collections WHERE id = ?1",
        params![id],
        map_collection,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_collections(state: State<'_, DbState>) -> Result<Vec<Collection>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let ws = active_workspace_id(&conn)?;
    fetch_collections(&conn, ws)
}

#[tauri::command]
pub fn create_collection(
    state: State<'_, DbState>,
    input: CreateCollectionInput,
) -> Result<Collection, String> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err("集合名称不能为空".into());
    }

    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let ws = input
        .workspace_id
        .unwrap_or(active_workspace_id(&conn)?);
    let base_url = input.base_url.trim().to_string();

    conn.execute(
        "INSERT INTO collections (parent_id, name, workspace_id, base_url) VALUES (?1, ?2, ?3, ?4)",
        params![input.parent_id, name, ws, base_url],
    )
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();
    get_collection_by_id(&conn, id)
}

#[tauri::command]
pub fn update_collection(
    state: State<'_, DbState>,
    input: UpdateCollectionInput,
) -> Result<Collection, String> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err("集合名称不能为空".into());
    }
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let n = conn
        .execute(
            "UPDATE collections SET name = ?1, base_url = ?2, updated_at = datetime('now') WHERE id = ?3",
            params![name, input.base_url.trim(), input.id],
        )
        .map_err(|e| e.to_string())?;
    if n == 0 {
        return Err("集合不存在".into());
    }
    get_collection_by_id(&conn, input.id)
}

#[tauri::command]
pub fn rename_collection(
    state: State<'_, DbState>,
    input: RenameCollectionInput,
) -> Result<Collection, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let existing = get_collection_by_id(&conn, input.id)?;
    let base_url = input
        .base_url
        .unwrap_or(existing.base_url)
        .trim()
        .to_string();
    drop(conn);
    update_collection(
        state,
        UpdateCollectionInput {
            id: input.id,
            name: input.name,
            base_url,
        },
    )
}

#[tauri::command]
pub fn delete_collection(state: State<'_, DbState>, id: i64) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    delete_collection_recursive(&conn, id)
}

fn delete_collection_recursive(conn: &rusqlite::Connection, id: i64) -> Result<(), String> {
    let mut stmt = conn
        .prepare("SELECT id FROM collections WHERE parent_id = ?1")
        .map_err(|e| e.to_string())?;
    let child_ids: Vec<i64> = stmt
        .query_map(params![id], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    drop(stmt);

    for child in child_ids {
        conn.execute(
            "DELETE FROM requests WHERE collection_id = ?1",
            params![child],
        )
        .map_err(|e| e.to_string())?;
        delete_collection_recursive(conn, child)?;
    }

    conn.execute(
        "DELETE FROM requests WHERE collection_id = ?1",
        params![id],
    )
    .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM collections WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_sidebar_tree(state: State<'_, DbState>) -> Result<Vec<TreeNode>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let ws = active_workspace_id(&conn)?;
    let collections = fetch_collections(&conn, ws)?;

    let mut req_stmt = conn
        .prepare(
            "SELECT r.id, r.collection_id, r.name, r.method
             FROM requests r
             LEFT JOIN collections c ON r.collection_id = c.id
             WHERE c.workspace_id = ?1 OR r.collection_id IS NULL
             ORDER BY r.sort_order, r.id",
        )
        .map_err(|e| e.to_string())?;
    let all_requests: Vec<(i64, Option<i64>, String, String)> = req_stmt
        .query_map(params![ws], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    drop(req_stmt);

    let col_ids: std::collections::HashSet<i64> = collections.iter().map(|c| c.id).collect();
    let scoped_requests: Vec<(i64, Option<i64>, String, String)> = all_requests
        .into_iter()
        .filter(|(_, cid, _, _)| cid.map(|id| col_ids.contains(&id)).unwrap_or(false))
        .collect();

    Ok(build_tree(None, &collections, &scoped_requests))
}

fn build_tree(
    parent_id: Option<i64>,
    collections: &[Collection],
    requests: &[(i64, Option<i64>, String, String)],
) -> Vec<TreeNode> {
    let mut nodes: Vec<TreeNode> = collections
        .iter()
        .filter(|c| c.parent_id == parent_id)
        .map(|c| {
            let mut children = build_tree(Some(c.id), collections, requests);
            for (rid, cid, name, method) in requests {
                if *cid == Some(c.id) {
                    children.push(TreeNode {
                        id: format!("req-{rid}"),
                        label: name.clone(),
                        node_type: "request".into(),
                        collection_id: Some(c.id),
                        request_id: Some(*rid),
                        method: Some(method.clone()),
                        base_url: None,
                        children: vec![],
                    });
                }
            }
            TreeNode {
                id: format!("col-{}", c.id),
                label: c.name.clone(),
                node_type: "collection".into(),
                collection_id: Some(c.id),
                request_id: None,
                method: None,
                base_url: Some(c.base_url.clone()),
                children,
            }
        })
        .collect();

    if parent_id.is_none() {
        for (rid, cid, name, method) in requests {
            if cid.is_none() {
                nodes.push(TreeNode {
                    id: format!("req-{rid}"),
                    label: name.clone(),
                    node_type: "request".into(),
                    collection_id: None,
                    request_id: Some(*rid),
                    method: Some(method.clone()),
                    base_url: None,
                    children: vec![],
                });
            }
        }
    }

    nodes
}
