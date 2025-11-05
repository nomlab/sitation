use futures::future::join_all;
use html5ever::parse_document;
use html5ever::tendril::TendrilSink;
use markup5ever_rcdom::{Handle, NodeData, RcDom};
use std::sync::Arc;
use std::time::Duration;
use std::{collections::HashMap, io::Cursor};
use tokio::sync::Mutex;
use tokio::sync::Semaphore;
use tokio::time::sleep;
use url::Url;

static PERMITS: Semaphore = Semaphore::const_new(6);

// This is not proper HTML serialization, of course.

fn get_links(depth: usize, handle: &Handle) -> Vec<String> {
    let node = handle;
    let mut links: Vec<String> = vec![];

    if let NodeData::Element { ref attrs, .. } = node.data {
        for attr in attrs.borrow().iter() {
            if attr.name.local.to_string() == "href" || attr.name.local.to_string() == "src" {
                links.push(attr.value.to_string());
            }
        }
    }

    for child in node.children.borrow().iter() {
        let mut child_links = get_links(depth + 1, child);
        links.append(&mut child_links);
    }

    links
}

fn is_descendant(base: &Url, target: &Url) -> bool {
    let res = base.make_relative(target);
    match res {
        Some(rel_path) if rel_path.contains("..") => false,
        Some(_rel_path) => true,
        None => false,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let top_url = Url::parse("https://gc.cs.okayama-u.ac.jp/lab/nom/").unwrap();
    let stats: PageStats = Arc::new(Mutex::new(HashMap::new()));
    let stats_clone = stats.clone();
    let _task = tokio::spawn(async move {
        loop {
            print_stats(&stats_clone).await;
            sleep(Duration::from_secs(1)).await;
        }
    });
    get_stat_recursive(top_url.clone(), &top_url, stats.clone()).await;

    println!("{:?}", stats.lock().await);

    Ok(())
}

async fn print_stats(stats: &PageStats) {
    let stats = stats.lock().await;
    println!("Current number of URLs: {}", stats.len());
}

#[derive(Debug)]
struct PageStat {
    n_urls: usize,
}

// type PageStats = Arc<Mutex<HashMap<Url, PageStat>>>;
type PageStats = Arc<Mutex<HashMap<Url, PageStat>>>;

async fn get_stat_recursive(current_url: Url, top_url: &Url, stats: PageStats) {
    {
        let mut stats = stats.lock().await;
        if stats.get(&current_url).is_some() {
            eprintln!("Already visited: {}", current_url);
            return;
        } else {
            let stat = PageStat { n_urls: 0 };
            stats.insert(current_url.clone(), stat);
            println!("Visiting: {}", current_url);
        }
    }

    // Fetch the web page specified by current_url
    let resp = {
        let _permit = PERMITS.acquire().await.unwrap();
        reqwest::get(current_url.as_str())
            .await
            .unwrap()
            .text()
            .await
            .unwrap()
    };
    let mut resp = Cursor::new(resp);

    // Parse the HTML document
    let dom = parse_document(RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut resp)
        .unwrap();

    // Extract links from the document
    let urls = get_links(0, &dom.document);

    // Insert the page statistics into the shared stats map
    let stat = PageStat { n_urls: urls.len() };
    {
        let mut stats = stats.lock().await;
        stats.insert(current_url.clone(), stat);
    }

    // Process each link found on the page
    let mut tasks = vec![];
    for url in urls {
        // Check if the URL is a descendant of the top_url
        let url = current_url.join(url.as_str()).unwrap();
        if !is_descendant(top_url, &url) {
            continue;
        }

        // Check if the URL has already been processed
        let url_is_new = {
            let stats = stats.lock().await;
            !stats.contains_key(&url)
        };

        // If the URL is new, spawn a new task to process it recursively
        if url_is_new {
            let next_url = url.clone();
            let stats = stats.clone();
            let task = get_stat_recursive(next_url, top_url, stats);
            tasks.push(task);
        }
    }

    join_all(tasks).await;
}
