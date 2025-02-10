use crate::errors::AppResult;
use lopdf::{Document, Object};
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;

pub struct LoPdfProcessor;

impl LoPdfProcessor {
    /// Extracts text from a PDF file.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the PDF file.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the extracted text as a `String` if successful,
    /// or an `Error` if an error occurred during the extraction process.
    pub async fn extract_text<T: AsRef<std::path::Path>>(path: T) -> AppResult<String> {
        let doc = Document::load_filtered(path, filter_func).await?;
        let pages = doc
            .get_pages()
            .into_par_iter()
            .map(|(page_num, _): (u32, _)| {
                doc.extract_text(&[page_num])
                    .unwrap_or_default()
                    .split('\n')
                    .map(|s| s.trim_end().to_string())
                    .collect::<String>()
            })
            .collect::<Vec<_>>();

        Ok(pages.join(""))
    }
}

static IGNORE: &[&[u8]] = &[
    b"Length",
    b"BBox",
    b"FormType",
    b"Matrix",
    b"Type",
    b"XObject",
    b"Subtype",
    b"Filter",
    b"ColorSpace",
    b"Width",
    b"Height",
    b"BitsPerComponent",
    b"Length1",
    b"Length2",
    b"Length3",
    b"PTEX.FileName",
    b"PTEX.PageNumber",
    b"PTEX.InfoDict",
    b"FontDescriptor",
    b"ExtGState",
    b"MediaBox",
    b"Annot",
];

fn filter_func(object_id: (u32, u16), object: &mut Object) -> Option<((u32, u16), Object)> {
    if IGNORE.contains(&object.type_name().unwrap_or_default()) {
        return None;
    }
    if let Ok(d) = object.as_dict_mut() {
        d.remove(b"Producer");
        d.remove(b"ModDate");
        d.remove(b"Creator");
        d.remove(b"ProcSet");
        d.remove(b"Procset");
        d.remove(b"XObject");
        d.remove(b"MediaBox");
        d.remove(b"Annots");
        if d.is_empty() {
            return None;
        }
    }
    Some((object_id, object.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_extract_text() {
        let pdf_file = shellexpand::full("~/Downloads/CISDigital®工业互联网平台（V3.0）产品操作手册-工业时序数据存算平台-V1.0.pdf")
            .unwrap()
            .to_string();
        let text = LoPdfProcessor::extract_text(pdf_file).await.unwrap();
        println!("{:?}", text);
    }
}
