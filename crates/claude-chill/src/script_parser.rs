pub fn find_script_header_end(data: &[u8]) -> usize {
    if !data.starts_with(b"Script started on") {
        return 0;
    }
    for (i, &byte) in data.iter().enumerate() {
        if byte == b'\n' {
            return i + 1;
        }
    }
    0
}

pub fn find_script_footer_start(data: &[u8]) -> usize {
    let footer_marker = b"\nScript done on";
    for i in (0..data.len().saturating_sub(footer_marker.len())).rev() {
        if &data[i..i + footer_marker.len()] == footer_marker {
            return i;
        }
    }
    data.len()
}

pub fn strip_script_wrapper(data: &[u8]) -> &[u8] {
    let mut result = data;

    loop {
        let header_end = find_script_header_end(result);
        if header_end == 0 {
            break;
        }
        result = &result[header_end..];
    }

    loop {
        let footer_start = find_script_footer_start(result);
        if footer_start >= result.len() {
            break;
        }
        result = &result[..footer_start];
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_header_end_no_header() {
        let data = b"hello world";
        assert_eq!(find_script_header_end(data), 0);
    }

    #[test]
    fn test_find_header_end_with_header() {
        let data = b"Script started on 2024-01-01\ncontent here";
        assert_eq!(find_script_header_end(data), 29);
    }

    #[test]
    fn test_find_footer_start_no_footer() {
        let data = b"hello world";
        assert_eq!(find_script_footer_start(data), data.len());
    }

    #[test]
    fn test_find_footer_start_with_footer() {
        let data = b"content here\nScript done on 2024-01-01";
        assert_eq!(find_script_footer_start(data), 12);
    }

    #[test]
    fn test_strip_script_wrapper() {
        let data = b"Script started on 2024-01-01\ncontent\nScript done on 2024-01-01";
        let stripped = strip_script_wrapper(data);
        assert_eq!(stripped, b"content");
    }
}
