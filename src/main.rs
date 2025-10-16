use std::{collections::HashMap, io::Cursor};
use html5ever::parse_document;
use html5ever::tendril::TendrilSink;
use markup5ever_rcdom::{Handle, NodeData, RcDom};
use url::Url;

// This is not proper HTML serialization, of course.

fn get_links(depth: usize, handle: &Handle) -> Vec<String> {
    let node = handle;
    let mut links: Vec<String> = vec![];

    if let NodeData::Element { ref attrs, ..} = node.data {
        for attr in attrs.borrow().iter() {
            if attr.name.local.to_string() == "href" || attr.name.local.to_string() == "src" {
                links.push(attr.value.to_string());
            }
        }
    }

    for child in node.children.borrow().iter() {
        let mut child_links = get_links(depth + 1, child);
        links.append(&mut child_links);
    };

    links
}

fn is_descendant(base: &Url, target:&Url) -> bool {
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
    let mut urls: HashMap<Url, bool> = HashMap::new();
    let resp = reqwest::get(top_url.as_str())
        .await?
        .text()
        .await?;

    let mut resp = Cursor::new(resp);

    let dom = parse_document(RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut resp)
        .unwrap();

    let links = get_links(0, &dom.document);
    for link in links {
        let url = top_url.join(link.as_str()).unwrap();

        if !is_descendant(&top_url, &url) {
            continue;
        }

        if let Some(_) = urls.insert(url.clone(), true){
            println!("{url}(dup)");
        }else {
            println!("{url}");
        };
    }

    Ok(())
}
