use docx_rs::*;

use std::fs::File;
use std::io::Read;

use crate::errors::AppResult;
use docx_rs::read_docx;

pub struct DocxRsProcessor;

impl DocxRsProcessor {
    /// Extracts text from a Docx file.
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
        let mut file = File::open(path)?;

        let mut buf = vec![];
        file.read_to_end(&mut buf)?;

        let res = read_docx(&buf)?;
        let mut text = String::new();
        let children = res.document.children;

        for i in children {
            match i {
                DocumentChild::Paragraph(s) => {
                    parse_paragraph(s, &mut text);
                }
                DocumentChild::Table(s) => {
                    parse_table(s, &mut text);
                }
                _ => {}
            }
        }

        Ok(text)
    }
}

fn parse_table(s: Box<Table>, text: &mut String) {
    for ele in s.rows {
        match ele {
            TableChild::TableRow(tr) => {
                for ele2 in tr.cells {
                    match ele2 {
                        TableRowChild::TableCell(tc) => {
                            for ele3 in tc.children {
                                match ele3 {
                                    TableCellContent::Paragraph(tp) => {
                                        parse_paragraph(Box::new(tp), text);
                                    }
                                    TableCellContent::Table(tt) => {
                                        for row_ele in tt.rows {
                                            match row_ele {
                                                TableChild::TableRow(ttr) => {
                                                    for ttr_ele in ttr.cells {
                                                        match ttr_ele {
                                                            TableRowChild::TableCell(ttr_tc) => {
                                                                for ele in ttr_tc.children {
                                                                    match ele {
                                                                        TableCellContent::Paragraph(p) => {
                                                                            parse_paragraph(Box::new(p), text);
                                                                        }
                                                                        TableCellContent::Table(t) => {
                                                                            parse_table(Box::new(t), text);
                                                                        }
                                                                        _ => {}
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn parse_paragraph(s: Box<Paragraph>, text: &mut String) {
    for ele in s.children {
        match ele {
            ParagraphChild::Run(r) => {
                for ele2 in r.children {
                    match ele2 {
                        RunChild::Text(t) => {
                            text.push_str(&t.text);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_extract_text() {
        let pdf_file = shellexpand::full("~/Downloads/CISDigital®工业互联网平台（V3.0）产品操作手册-工业时序数据存算平台-V1.0 .docx")
            .unwrap()
            .to_string();
        let text = DocxRsProcessor::extract_text(pdf_file).await.unwrap();
        println!("{:?}", text);
    }
}
