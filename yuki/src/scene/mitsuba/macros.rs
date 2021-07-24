#[macro_export]
macro_rules! try_find_attr {
    ($attributes:expr, $name_str:expr) => {{
        let mut value = None;
        for attr in $attributes {
            if attr.name.local_name.as_str() == $name_str {
                value = Some(&attr.value);
            }
        }
        value
    }};
}

#[macro_export]
macro_rules! find_attr {
    ($attributes:expr, $name_str:expr) => {{
        match crate::try_find_attr!($attributes, $name_str) {
            Some(v) => v,
            None => return Err(format!("Could not find element attribute '{}'", $name_str).into()),
        }
    }};
}

#[macro_export]
/// Pumps messages, calling start_body for each StartElement.
/// 'return's errors for unexpected data blocks.
/// Breaks when an unmatched EndElement is encountered.
///
/// start_body has a signature of (name: &OwnedName, attributes: Vec<OwnedAttribute>, ignore_level: &mut Option<u32>) -> Result<()>
/// 'ignore_level = Some(0)' can be set to skip the current element and it's children.
/// 'level' should be decremented after a recursive parser call returns to match the correct level (caller won't see EndElement)
macro_rules! parse_element {
    ($parser:ident, $indent:ident, $start_body:expr) => {
        let mut level = 0;
        let mut ignore_level: Option<u32> = None;
        loop {
            match $parser.next() {
                Ok(evt) => match evt {
                    xml::reader::XmlEvent::StartDocument { .. } => unreachable!(),
                    xml::reader::XmlEvent::StartElement {
                        name, attributes, ..
                    } => {
                        if let None = ignore_level {
                            yuki_trace!("{}Begin: {}", $indent, name);
                            $indent += "  ";
                            yuki_trace!("{}Attributes", $indent);
                            $indent += "  ";
                            for xml::attribute::OwnedAttribute { name, value } in &attributes {
                                yuki_trace!("{}{}: {}", $indent, name, value);
                            }
                            $indent.truncate($indent.len() - 2);
                        }

                        if let None = ignore_level {
                            $start_body(&name, attributes, &mut level, &mut ignore_level)?;
                        }

                        level += 1;

                        if let Some(l) = ignore_level {
                            if l == 0 {
                                yuki_info!("Element '{}' ignored", name);
                            }
                            ignore_level = Some(l + 1);
                        }
                    }
                    xml::reader::XmlEvent::EndElement { name } => {
                        if let Some(l) = ignore_level {
                            let level_after = l - 1;
                            if level_after > 0 {
                                ignore_level = Some(l - 1);
                            } else {
                                ignore_level = None;
                            }
                        }

                        if ignore_level == None || ignore_level == Some(0) {
                            $indent.truncate($indent.len() - 2);
                            yuki_trace!("{}End: {}", $indent, name);
                        }

                        level -= 1;
                        if level < 0 {
                            break;
                        }
                    }
                    xml::reader::XmlEvent::ProcessingInstruction { name, .. } => {
                        return Err(format!("Unexpected processing instruction: {}", name).into())
                    }
                    xml::reader::XmlEvent::CData(data) => {
                        return Err(format!("Unexpected CDATA: {}", data).into())
                    }
                    xml::reader::XmlEvent::Comment(_) => (),
                    xml::reader::XmlEvent::Characters(chars) => {
                        return Err(format!("Unexpected characters outside tags: {}", chars).into())
                    }
                    xml::reader::XmlEvent::Whitespace(_) => (),
                    xml::reader::XmlEvent::EndDocument => unreachable!(),
                },
                Err(err) => {
                    yuki_error!("XML error: {}", err);
                    break;
                }
            }
        }
    };
}
