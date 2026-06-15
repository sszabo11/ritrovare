use anyhow::Result;
use tabby::{
    browser::Browser, local::LocalDB, logs::init_logging, model::Model, screen::Screen,
    spinners::Spinner, utils::filter_tabs,
};

#[tokio::main]
async fn main() -> Result<()> {
    init_logging();

    let model = Model::new("gemma4");

    let mut browser = Browser::new();

    let mut screen = Screen::new();
    screen.draw()?;

    let mut local = LocalDB::new();

    local.init_db()?;
    //{
    //    let all_tabs = browser.fetch().expect("Failed to fetch tabs");

    //    // Embed new tabs for saving
    //    let embedded = model.embed_tabs(&all_tabs).await?;

    //    // Save new tabs in vector DB
    //    local.save_new_tabs(&embedded, &all_tabs)?;
    //    browser.set(all_tabs);
    //}

    let last_saved_tab = local.last_saved_tab()?;
    println!("last saved tab: {:?}", last_saved_tab);

    // Get tabs since last fetch
    let new_tabs = browser.fetch_latest(last_saved_tab)?;
    println!("New tabs: {:?}", new_tabs.len());

    let filtered_tabs = filter_tabs(new_tabs);

    // Embed new tabs for saving
    let embedded = model.embed_tabs(&filtered_tabs).await?;
    println!("Inserting {} tabs", filtered_tabs.len());
    assert_eq!(embedded.len(), filtered_tabs.len());

    // Save new tabs in vector DB
    local.save_new_tabs(&embedded, &filtered_tabs)?;

    // Update state to ready

    //for i in (browser.history.len() - 100)..browser.history.len() {
    //    println!("{:?}", browser.history[i])
    //}

    Ok(())
}
