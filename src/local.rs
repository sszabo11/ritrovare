use anyhow::Result;
use rusqlite::Connection;

use crate::{browser::Tab, utils::parse_val_str};

pub struct LocalDB {
    api: Connection,
}

const PLACES_PATH: &str = "Downloads/0qh2f0lc.Default (alpha)/places.sqlite";
impl LocalDB {
    pub fn new() -> Self {
        let conn = Connection::open("local.sqlite").unwrap();

        Self { api: conn }
    }
    //pub id: i32,
    //pub url: String,
    //pub title: String,
    //pub visit_count: i32,
    //pub last_visit_date: i64,
    //pub desc: String,
    //pub created_at: i64,
    //pub updated_at: i64,
    //pub total_view_time: i32,
    //pub typing_time: i32,
    //pub scrolling_time: i32,
    //pub scrolling_distance: i32,

    pub fn init_db(&self) -> Result<()> {
        let res = self.api.execute(
            "
            CREATE TABLE IF NOT EXISTS tabs (
                id                INTEGER PRIMARY KEY,
                url               TEXT UNIQUE NOT NULL,
                title             TEXT NOT NULL DEFAULT '',
                visit_count       INTEGER NOT NULL DEFAULT 0,
                last_visit_date   INTEGER NOT NULL DEFAULT 0,
                desc              TEXT NOT NULL DEFAULT '',
                created_at        INTEGER NOT NULL DEFAULT 0,
                updated_at        INTEGER NOT NULL DEFAULT 0,
                total_view_time   INTEGER NOT NULL DEFAULT 0,
                typing_time       INTEGER NOT NULL DEFAULT 0,
                scrolling_time    INTEGER NOT NULL DEFAULT 0,
                scrolling_distance INTEGER NOT NULL DEFAULT 0,
                embedding         BLOB NOT NULL
            );
            ",
            (),
        )?;

        Ok(())
    }

    pub fn last_saved_tab(&self) -> Result<Option<Tab>> {
        let mut last = self
            .api
            .prepare("SELECT * FROM tabs ORDER BY id DESC LIMIT 1")?;

        let data = last.query_map([], |row| {
            Ok(Tab {
                id: row.get(0)?,
                url: row.get(1)?,
                title: parse_val_str(row.get(2)?),
                visit_count: row.get(3)?,
                last_visit_date: row.get(4)?,
                desc: parse_val_str(row.get(5)?),
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
                total_view_time: row.get(8)?,
                typing_time: row.get(9)?,
                scrolling_time: row.get(10)?,
                scrolling_distance: row.get(11)?,
            })
        })?;

        let tab = data.into_iter().map(|r| r.unwrap()).collect::<Vec<Tab>>();

        //let tab: Option<Tab> = data.into_iter().flatten().next();

        Ok(Some(tab[0].clone()))
    }

    pub fn upsert_tab(&self, conn: &Connection, tab: &Tab, embedding: &Vec<f32>) -> Result<()> {
        let embedding_bytes: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();

        conn.execute(
            "INSERT INTO tabs (
            url, title, visit_count, last_visit_date, desc,
            created_at, updated_at, total_view_time,
            typing_time, scrolling_time, scrolling_distance, embedding
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
        ON CONFLICT(url) DO UPDATE SET
            title               = excluded.title,
            visit_count         = excluded.visit_count,
            last_visit_date     = excluded.last_visit_date,
            desc                = excluded.desc,
            updated_at          = excluded.updated_at,
            total_view_time     = excluded.total_view_time,
            typing_time         = excluded.typing_time,
            scrolling_time      = excluded.scrolling_time,
            scrolling_distance  = excluded.scrolling_distance,
            embedding           = excluded.embedding
            ",
            rusqlite::params![
                tab.url,
                tab.title,
                tab.visit_count,
                tab.last_visit_date,
                tab.desc,
                tab.created_at,
                tab.updated_at,
                tab.total_view_time,
                tab.typing_time,
                tab.scrolling_time,
                tab.scrolling_distance,
                embedding_bytes
            ],
        )?;
        Ok(())
    }
    pub fn save_new_tabs(&mut self, embeddings: &Vec<Vec<f32>>, tabs: &Vec<Tab>) -> Result<()> {
        let tx = self.api.unchecked_transaction()?;
        for (i, tab) in tabs.iter().enumerate() {
            self.upsert_tab(&tx, tab, &embeddings[i])?;
        }
        tx.commit()?;
        Ok(())
    }
}
