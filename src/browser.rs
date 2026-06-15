use anyhow::Result;
use rusqlite::{Connection, ffi::Error, types::Value};

use crate::utils::parse_val_str;

#[derive(Debug, Clone)]
pub struct Tab {
    pub id: i32,
    pub url: String,
    pub title: String,
    pub visit_count: i64,
    pub last_visit_date: i64,
    pub desc: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub total_view_time: i64,
    pub typing_time: i32,
    pub scrolling_time: i32,
    pub scrolling_distance: i32,
}
/*
       [0] id: 16574
       [1] url: https://www.google.com/search?client=firefox-b-d&q=funding+fee+biannce
       [2] title: funding fee biannce - Google Search
       [3] rev_host: moc.elgoog.www.
       [4] visit_count: 1
       [5] hidden: 0
       [6] typed: 1
       [7] frecency: 20
       [8] last_visit_date: 1759816584945000
       [9] guid: wCeaYsonHgHA
       [10] foreign_count: 0
       [11] url_hash: 47360567069366
       [12] description: NULL
       [13] preview_image_url: NULL
       [14] site_name: NULL
       [15] origin_id: 6
       [16] recalc_frecency: 0
       [17] alt_frecency: NULL
       [18] recalc_alt_frecency: 1
       [19] id: 16574
       [20] place_id: 72
       [21] referrer_place_id: 96
       [22] created_at: 1754300684350
       [23] updated_at: 1754300685193
       [24] total_view_time: 258
       [25] typing_time: 0
       [26] key_presses: 0
       [27] scrolling_time: 0
       [28] scrolling_distance: 0
       [29] document_type: 0
       [30] search_query_id: NULL
*/

pub struct Browser {
    pub history: Vec<Tab>,
    api: Connection,
}

const PLACES_PATH: &str = "Downloads/0qh2f0lc.Default (alpha)/places.sqlite";
impl Browser {
    pub fn new() -> Self {
        let home = std::env::var("HOME").expect("HOME not set");
        let src_path = format!("{}/{}", home, PLACES_PATH);

        let conn = Connection::open(format!("file://{}?immutable=1&mode=ro", src_path)).unwrap();

        Self {
            history: vec![],
            api: conn,
        }
    }

    pub fn set(&mut self, tabs: Vec<Tab>) {
        self.history = tabs;
    }

    pub fn fetch_latest(&mut self, last_tab: Option<Tab>) -> Result<Vec<Tab>> {
        let last_tab = last_tab.expect("No last tab, cannot fetch latest");
        log::info!("last tab id: {}", last_tab.id);

        let mut moz_places = self.api.prepare(
            "SELECT * FROM moz_places 
            INNER JOIN moz_places_metadata 
            ON moz_places.id == moz_places_metadata.id WHERE moz_places_metadata.created_at > ?1",
        )?;

        let data = moz_places.query_map([last_tab.created_at], |row| {
            Ok(Tab {
                id: row.get(0)?,
                url: row.get(1)?,
                title: parse_val_str(row.get(2)?),
                visit_count: row.get(4)?,
                last_visit_date: row.get(8)?,
                desc: parse_val_str(row.get(12)?),
                created_at: row.get(22)?,
                updated_at: row.get(23)?,
                total_view_time: row.get(24)?,
                typing_time: row.get(25)?,
                scrolling_time: row.get(27)?,
                scrolling_distance: row.get(28)?,
            })
        })?;

        let tabs: Vec<Tab> = data.into_iter().map(|r| r.unwrap()).collect();

        Ok(tabs)
    }
    pub fn fetch(&mut self) -> Result<Vec<Tab>> {
        let mut moz_places = self.api.prepare(
            "SELECT * FROM moz_places 
            INNER JOIN moz_places_metadata 
            ON moz_places.id == moz_places_metadata.id;",
        )?;

        let data = moz_places.query_map([], |row| {
            Ok(Tab {
                id: row.get(0)?,
                url: row.get(1)?,
                title: parse_val_str(row.get(2)?),
                visit_count: row.get(4)?,
                last_visit_date: row.get(8)?,
                desc: parse_val_str(row.get(12)?),
                created_at: row.get(22)?,
                updated_at: row.get(23)?,
                total_view_time: row.get(24)?,
                typing_time: row.get(25)?,
                scrolling_time: row.get(27)?,
                scrolling_distance: row.get(28)?,
            })
        })?;

        let tabs: Vec<Tab> = data.into_iter().map(|r| r.unwrap()).collect();

        Ok(tabs)
    }

    pub fn analyze_records(&self) -> Result<()> {
        let mut moz_places = self.api.prepare("SELECT * FROM moz_places INNER JOIN moz_places_metadata ON moz_places.id == moz_places_metadata.id;")?;

        let tables2 = moz_places.query_map([], |row| {
            let col_names: Vec<String> = row
                .as_ref()
                .column_names()
                .iter()
                .map(|s| s.to_string())
                .collect();
            loop {
                for (i, name) in col_names.iter().enumerate() {
                    match row.get::<usize, Value>(i) {
                        Ok(val) => match &val {
                            Value::Null => println!("[{}] {}: NULL", i, name),
                            Value::Integer(n) => println!("[{}] {}: {}", i, name, n),
                            Value::Real(f) => println!("[{}] {}: {}", i, name, f),
                            Value::Text(s) => println!("[{}] {}: {}", i, name, s),
                            Value::Blob(b) => {
                                println!("[{}] {}: <blob {} bytes>", i, name, b.len())
                            }
                        },
                        Err(err) => {
                            println!("Err: {}", err);
                            break;
                        }
                    }
                }
            }
            println!();
            Ok(())
        })?;

        for _table in tables2 {}

        Ok(())
    }
}

fn parse_val<T: From<Value> + Default>(val: Value) -> T {
    match val {
        Value::Null => T::default(),
        v => T::from(v),
    }
}
