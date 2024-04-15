use eyre::eyre;
use eyre::Result;
use html5ever::{ns, namespace_url};
use html5ever::{QualName, LocalName};
use hyper::Uri;
use lazy_static::lazy_static;
use super::HtmlMod;


lazy_static! {
    static ref QUAL_NAME: QualName = QualName::new(
        None,
        ns!(html),
        LocalName::from("nib:script")
    );
}

pub struct ScriptMod {
    page_uri: Uri
}

impl ScriptMod {
    pub fn new(page_uri: Uri) -> Self {
        ScriptMod {
            page_uri
        }
    }
}

impl HtmlMod for ScriptMod {
    fn modify(&self, html: super::Html) -> Result<super::Html> {
        for node in html.descendants() {
            if let Some(el) = node.as_element() {
                if el.name == *QUAL_NAME {
                    let contents = node.text_contents();

                    // detach script
                    node.detach()
                }
            }
        }

        Ok(html)
    }
}
