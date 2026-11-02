//! Direct child order of an XML playlist-tree folder, read from source bytes.

pub(crate) fn direct_xml_typed_child_order(
    source: &std::path::Path,
    wanted: &[String],
) -> Result<Vec<String>, String> {
    use quick_xml::{Reader, events::Event};
    let bytes = std::fs::read(source).map_err(|e| e.to_string())?;
    let mut reader = Reader::from_reader(bytes.as_slice());
    let mut buf = Vec::new();
    let mut folders = Vec::<String>::new();
    let mut frames = Vec::<bool>::new();
    let mut result = Vec::new();
    loop {
        match reader
            .read_event_into(&mut buf)
            .map_err(|e| e.to_string())?
        {
            Event::Start(e) if e.name().as_ref() == b"NODE" => {
                let (mut kind, mut name) = (String::new(), String::new());
                for attr in e.attributes().with_checks(false) {
                    let attr = attr.map_err(|e| e.to_string())?;
                    let value = attr
                        .decoded_and_normalized_value(
                            quick_xml::XmlVersion::Implicit1_0,
                            reader.decoder(),
                        )
                        .map_err(|e| e.to_string())?
                        .into_owned();
                    match attr.key.as_ref() {
                        b"Type" => kind = value,
                        b"Name" => name = value,
                        _ => {}
                    }
                }
                if folders == wanted {
                    result.push(format!("{kind}:{name}"));
                }
                let nested = kind == "0" && !frames.is_empty();
                if nested {
                    folders.push(name);
                }
                frames.push(nested);
            }
            Event::Empty(e) if e.name().as_ref() == b"NODE" => {
                let (mut kind, mut name) = (String::new(), String::new());
                for attr in e.attributes().with_checks(false) {
                    let attr = attr.map_err(|e| e.to_string())?;
                    let value = attr
                        .decoded_and_normalized_value(
                            quick_xml::XmlVersion::Implicit1_0,
                            reader.decoder(),
                        )
                        .map_err(|e| e.to_string())?
                        .into_owned();
                    match attr.key.as_ref() {
                        b"Type" => kind = value,
                        b"Name" => name = value,
                        _ => {}
                    }
                }
                if folders == wanted {
                    result.push(format!("{kind}:{name}"));
                }
            }
            Event::End(e) if e.name().as_ref() == b"NODE" => {
                if frames.pop().unwrap_or(false) {
                    folders.pop();
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(result)
}

pub(crate) fn direct_xml_node_order(
    source: &std::path::Path,
    wanted: &[String],
) -> Result<Vec<String>, String> {
    use quick_xml::{Reader, events::Event};
    let bytes = std::fs::read(source).map_err(|e| e.to_string())?;
    let mut reader = Reader::from_reader(bytes.as_slice());
    let mut buf = Vec::new();
    let mut folders: Vec<String> = Vec::new();
    let mut folder_frames: Vec<bool> = Vec::new();
    let mut result = Vec::new();
    loop {
        match reader
            .read_event_into(&mut buf)
            .map_err(|e| e.to_string())?
        {
            Event::Start(e) if e.name().as_ref() == b"NODE" => {
                let mut kind = None;
                let mut name = None;
                for attr in e.attributes().with_checks(false) {
                    let attr = attr.map_err(|e| e.to_string())?;
                    let value = attr
                        .decoded_and_normalized_value(
                            quick_xml::XmlVersion::Implicit1_0,
                            reader.decoder(),
                        )
                        .map_err(|e| e.to_string())?
                        .into_owned();
                    match attr.key.as_ref() {
                        b"Type" => kind = Some(value),
                        b"Name" => name = Some(value),
                        _ => {}
                    }
                }
                let kind = kind.ok_or("NODE missing Type")?;
                let name = name.ok_or("NODE missing Name")?;
                if folders == wanted {
                    result.push(name.clone());
                }
                let is_folder = kind == "0";
                if is_folder && !folder_frames.is_empty() {
                    folders.push(name);
                }
                folder_frames.push(is_folder && !folder_frames.is_empty());
            }
            Event::Empty(e) if e.name().as_ref() == b"NODE" => {
                let mut kind = None;
                let mut name = None;
                for attr in e.attributes().with_checks(false) {
                    let attr = attr.map_err(|e| e.to_string())?;
                    let value = attr
                        .decoded_and_normalized_value(
                            quick_xml::XmlVersion::Implicit1_0,
                            reader.decoder(),
                        )
                        .map_err(|e| e.to_string())?
                        .into_owned();
                    match attr.key.as_ref() {
                        b"Type" => kind = Some(value),
                        b"Name" => name = Some(value),
                        _ => {}
                    }
                }
                let name = name.ok_or("NODE missing Name")?;
                if folders == wanted {
                    result.push(name);
                }
                let _ = kind;
            }
            Event::End(e) if e.name().as_ref() == b"NODE" => {
                if folder_frames.pop().unwrap_or(false) {
                    folders.pop();
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(result)
}
