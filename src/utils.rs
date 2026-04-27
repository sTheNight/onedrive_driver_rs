use crate::{error::OneDriveApiError, state::AppState};
use percent_encoding::{AsciiSet, CONTROLS, percent_decode_str, utf8_percent_encode};

const GRAPH_DRIVE_ROOT_URL: &str = "https://graph.microsoft.com/v1.0/me/drive/root";
const GRAPH_DRIVE_ROOT_CHILDREN_URL: &str =
    "https://graph.microsoft.com/v1.0/me/drive/root/children";
const GRAPH_PATH_SEGMENT_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'`')
    .add(b'{')
    .add(b'}')
    .add(b'/')
    .add(b'\\')
    .add(b'^')
    .add(b'[')
    .add(b']')
    .add(b'|')
    .add(b':');

pub fn graph_children_url(state: &AppState, path: &str) -> Result<reqwest::Url, OneDriveApiError> {
    let segments = onedrive_path_segments(&state.root_path, path);

    if segments.is_empty() {
        return parse_graph_url(GRAPH_DRIVE_ROOT_CHILDREN_URL);
    }

    let encoded_path = encode_graph_path(&segments);
    parse_graph_url(&format!(
        "https://graph.microsoft.com/v1.0/me/drive/root:/{encoded_path}:/children"
    ))
}

pub fn graph_item_url(state: &AppState, path: &str) -> Result<reqwest::Url, OneDriveApiError> {
    let segments = onedrive_path_segments(&state.root_path, path);

    if segments.is_empty() {
        return parse_graph_url(GRAPH_DRIVE_ROOT_URL);
    }

    let encoded_path = encode_graph_path(&segments);
    parse_graph_url(&format!(
        "https://graph.microsoft.com/v1.0/me/drive/root:/{encoded_path}:"
    ))
}

fn onedrive_path_segments(root_path: &str, path: &str) -> Vec<String> {
    decode_path_segments(root_path)
        .into_iter()
        .chain(decode_path_segments(path))
        .collect()
}

fn decode_path_segments(path: &str) -> Vec<String> {
    percent_decode_str(path.trim_matches('/'))
        .decode_utf8_lossy()
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn encode_graph_path(segments: &[String]) -> String {
    segments
        .iter()
        .map(|segment| utf8_percent_encode(segment, GRAPH_PATH_SEGMENT_ENCODE_SET).to_string())
        .collect::<Vec<_>>()
        .join("/")
}

fn parse_graph_url(url: &str) -> Result<reqwest::Url, OneDriveApiError> {
    reqwest::Url::parse(url).map_err(|err| OneDriveApiError::GraphUrlBuild(err.to_string()))
}
