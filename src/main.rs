use std::io::Cursor;
use html5ever::parse_document;
use html5ever::tendril::TendrilSink;
use markup5ever_rcdom::{Handle, NodeData, RcDom};

// This is not proper HTML serialization, of course.

fn walk(depth: usize, handle: &Handle) {
    let node = handle;

    if let NodeData::Element { ref attrs, ..} = node.data {
        for attr in attrs.borrow().iter() {
            if attr.name.local.to_string() == "href" || attr.name.local.to_string() == "src" {
                println!("<{}>", attr.value);
            } 
        }   
    }

    for child in node.children.borrow().iter() {
        walk(depth + 1, child);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let resp = reqwest::get("https://gc.cs.okayama-u.ac.jp/lab/nom/")
        .await?
        .text()
        .await?;

    let mut resp = Cursor::new(resp);

    let dom = parse_document(RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut resp)
        .unwrap();
    walk(0, &dom.document);

    Ok(())
}