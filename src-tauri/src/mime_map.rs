use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocType {
    Document,
    Spreadsheet,
    Presentation,
}

#[derive(Clone, Debug)]
pub struct Mapping {
    pub source_mime: &'static str,
    pub google_mime: &'static str,
    pub doc_type: DocType,
}

pub fn mapping_for_file(path: &Path) -> Option<Mapping> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())?;

    Some(match ext.as_str() {
        "doc" => Mapping {
            source_mime: "application/msword",
            google_mime: "application/vnd.google-apps.document",
            doc_type: DocType::Document,
        },
        "docx" => Mapping {
            source_mime:
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            google_mime: "application/vnd.google-apps.document",
            doc_type: DocType::Document,
        },
        "odt" => Mapping {
            source_mime: "application/vnd.oasis.opendocument.text",
            google_mime: "application/vnd.google-apps.document",
            doc_type: DocType::Document,
        },
        "ods" => Mapping {
            source_mime: "application/vnd.oasis.opendocument.spreadsheet",
            google_mime: "application/vnd.google-apps.spreadsheet",
            doc_type: DocType::Spreadsheet,
        },
        "odp" => Mapping {
            source_mime: "application/vnd.oasis.opendocument.presentation",
            google_mime: "application/vnd.google-apps.presentation",
            doc_type: DocType::Presentation,
        },
        "rtf" => Mapping {
            source_mime: "application/rtf",
            google_mime: "application/vnd.google-apps.document",
            doc_type: DocType::Document,
        },
        "xls" => Mapping {
            source_mime: "application/vnd.ms-excel",
            google_mime: "application/vnd.google-apps.spreadsheet",
            doc_type: DocType::Spreadsheet,
        },
        "xlsx" => Mapping {
            source_mime:
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            google_mime: "application/vnd.google-apps.spreadsheet",
            doc_type: DocType::Spreadsheet,
        },
        "csv" | "tsv" => Mapping {
            source_mime: "text/csv",
            google_mime: "application/vnd.google-apps.spreadsheet",
            doc_type: DocType::Spreadsheet,
        },
        "ppt" => Mapping {
            source_mime: "application/vnd.ms-powerpoint",
            google_mime: "application/vnd.google-apps.presentation",
            doc_type: DocType::Presentation,
        },
        "pptx" => Mapping {
            source_mime:
                "application/vnd.openxmlformats-officedocument.presentationml.presentation",
            google_mime: "application/vnd.google-apps.presentation",
            doc_type: DocType::Presentation,
        },
        _ => return None,
    })
}

pub fn doc_type_from_google_mime(mime: &str) -> DocType {
    match mime {
        "application/vnd.google-apps.spreadsheet" => DocType::Spreadsheet,
        "application/vnd.google-apps.presentation" => DocType::Presentation,
        _ => DocType::Document,
    }
}

pub fn drive_url(doc_type: DocType, file_id: &str) -> String {
    let segment = match doc_type {
        DocType::Document => "document",
        DocType::Spreadsheet => "spreadsheets",
        DocType::Presentation => "presentation",
    };
    format!("https://docs.google.com/{}/d/{}/edit", segment, file_id)
}
