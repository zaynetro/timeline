use anyhow::Result;
use bolik_migrations::rusqlite::{params, Connection};
use bolik_proto::sync::doc_payload::DocSchema;

use crate::documents::{self, DbDocRow, DbDocRowMeta, ALL_LABEL_ID, BIN_LABEL_ID};

use self::card::CardView;

pub mod acl_doc;
pub mod card;
mod timeline_atom;

pub use timeline_atom::{EditCardOpts, PermanentDeleteOpts, TimelineAtom, TimelineCtx};

pub fn timeline_days(conn: &Connection, label_ids: Vec<String>) -> Result<Vec<String>> {
    let query = r#"
SELECT strftime('%Y-%m-%d', created_at) AS created_day
  FROM documents d
  JOIN card_index i ON d.id = i.id
 WHERE schema = ? AND i.label_ids MATCH ?
 GROUP BY created_day
 ORDER BY created_day DESC"#;

    let mut stmt = conn.prepare(query)?;
    let labels_query = build_labels_query(label_ids);
    let mut rows = stmt.query(params![DocSchema::CardV1 as i32, labels_query])?;

    let mut days = vec![];
    while let Some(row) = rows.next()? {
        days.push(row.get(0)?);
    }
    Ok(days)
}

pub fn timeline_by_day(
    conn: &Connection,
    day: &str,
    label_ids: Vec<String>,
) -> Result<TimelineDay> {
    // Select documents and optionally join with labels
    let query = r#"
    SELECT d.id, d.data, d.acl_data, d.created_at, d.edited_at, strftime('%Y-%m-%d', d.created_at) AS created_day,
           d2.id, d2.data, d2.created_at, d2.edited_at
      FROM documents d
      JOIN card_index i ON d.id = i.id
      LEFT JOIN documents d2 ON d.id || '/labels' = d2.id
     WHERE created_day = ? AND d.schema = ? AND i.label_ids MATCH ?
     ORDER BY d.created_at DESC"#;

    let mut stmt = conn.prepare(query)?;

    let labels_query = build_labels_query(label_ids);
    let mut rows = stmt.query(params![&day, DocSchema::CardV1 as i32, labels_query])?;
    let mut timeline_day = TimelineDay {
        day: day.to_string(),
        cards: vec![],
    };
    let yrs_client_id = 1; // Doesn't matter in this case

    while let Some(row) = rows.next()? {
        let id: String = row.get(0)?;
        let data: Vec<u8> = row.get(1)?;
        let acl_data: Vec<u8> = row.get(2)?;
        let created_at = row.get(3)?;
        let edited_at = row.get(4)?;
        let doc = documents::build_yrs_doc(yrs_client_id, &data)?;
        let acl = documents::build_yrs_doc(yrs_client_id, &acl_data)?;

        let labels_id: Option<String> = row.get(6)?;
        let labels_row = if let Some(labels_id) = labels_id {
            let data: Vec<u8> = row.get(7)?;
            let doc = documents::build_yrs_doc(yrs_client_id, &data)?;
            Some(DbDocRow {
                meta: DbDocRowMeta {
                    id: labels_id,
                    created_at: row.get(8)?,
                    edited_at: row.get(9)?,
                    schema: DocSchema::CardLabelsV1 as i32,
                    author_device_id: "".into(),
                    counter: 0,
                },
                yrs: doc,
                acl: yrs::Doc::new(),
            })
        } else {
            None
        };

        let view = CardView::from_db(
            DbDocRow {
                meta: DbDocRowMeta {
                    id,
                    created_at,
                    edited_at,
                    schema: DocSchema::CardV1 as i32,
                    author_device_id: "".into(),
                    counter: 0,
                },
                yrs: doc,
                acl,
            },
            labels_row,
        )
        .0;
        timeline_day.cards.push(view);
    }

    Ok(timeline_day)
}

pub struct TimelineDay {
    pub day: String,
    pub cards: Vec<CardView>,
}

pub fn index_card(conn: &Connection, card: &CardView) -> Result<()> {
    let mut label_ids = card.labels.iter().map(|l| l.id.clone()).collect::<Vec<_>>();

    // Card is not deleted. Inject "all" label.
    if !label_ids.iter().any(|id| id == BIN_LABEL_ID) {
        label_ids.push(ALL_LABEL_ID.to_string());
    }

    let labels_str = label_ids.join(",");
    conn.execute("DELETE FROM card_index WHERE id = ?", [&card.id])?;
    conn.execute(
        "INSERT INTO card_index (id, text, label_ids) VALUES (?, ?, ?)",
        params![card.id, "", labels_str],
    )?;
    Ok(())
}

/// Build FTS matching for label ids.
/// Queries:
/// 1. No labels: `"bolik-all"`
/// 2. Deleted:   `"one" AND "bolik-deleted"`
/// 3. By labels: `"one" AND "two" NOT "bolik-deleted"`
fn build_labels_query(label_ids: Vec<String>) -> String {
    fn wrap(s: &str) -> String {
        format!(r#""{}""#, s)
    }

    fn wrap_vec(v: Vec<String>) -> String {
        v.iter()
            .map(|id| wrap(id))
            .collect::<Vec<_>>()
            .join(" AND ")
    }

    if label_ids.is_empty() {
        // No labels
        wrap(ALL_LABEL_ID)
    } else if label_ids.iter().any(|id| id == BIN_LABEL_ID) {
        // Deleted (moved to bin)
        wrap_vec(label_ids)
    } else {
        // By labels
        format!("{} NOT {}", wrap_vec(label_ids), wrap(BIN_LABEL_ID))
    }
}
