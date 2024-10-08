#![forbid(unsafe_code)]

use std::collections::HashMap;

////////////////////////////////////////////////////////////////////////////////

pub type IniFile = HashMap<String, HashMap<String, String>>;

pub fn parse(content: &str) -> IniFile {
    let mut result = HashMap::new();
    let mut current_section_title: Option<&str> = None;

    for mut line in content.lines() {
        line = line.trim();

        if line.starts_with('[') {
            current_section_title = Some(parse_section_title(line));

            if !result.contains_key(current_section_title.unwrap()) {
                result.insert(current_section_title.unwrap().to_string(), HashMap::new());
            }
        } else if !line.is_empty() {
            let pair = parse_value_pair(line);

            assert!(current_section_title.is_some());
            let map = result.get_mut(current_section_title.unwrap());

            assert!(map.is_some());
            let map: &mut HashMap<String, String> = map.unwrap();

            map.insert(pair.key.to_string(), pair.value.to_string());
        }
    }

    result
}

#[derive(Debug)]
struct ValuePair<'a> {
    key: &'a str,
    value: &'a str,
}

fn parse_value_pair(line: &str) -> ValuePair {
    let mut iter = line.split('=');

    let key = iter.next().unwrap().trim();

    let value = match iter.next() {
        Some(val) => val.trim(),
        None => "",
    };

    assert!(iter.next().is_none());

    ValuePair { key, value }
}

fn parse_section_title(line: &str) -> &str {
    assert!(line.ends_with(']'));

    let title = &line[1..line.len() - 1];

    assert_eq!(title.find('['), None);
    assert_eq!(title.find(']'), None);

    assert!(!title.is_empty());

    title
}
