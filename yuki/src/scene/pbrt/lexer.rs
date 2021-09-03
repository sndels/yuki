use std::convert::TryFrom;
use strum::ToString;

// Based on the rules laid out in the old pbrtlex.ll of pbrt-v3

#[derive(Debug)]
pub struct LexerError {
    pub error_type: LexerErrorType,
    pub location: FileLocation,
}

impl LexerError {
    fn new(error_type: LexerErrorType, location: FileLocation) -> Self {
        Self {
            error_type,
            location,
        }
    }
}

#[derive(Debug, ToString)]
pub enum LexerErrorType {
    EndOfInput,
    UnexpectedEndOfInput,
    UnterminatedString,
    InvalidNumber,
    UnknownIdentifier(String),
}

impl std::fmt::Display for LexerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let type_str = match &self.error_type {
            LexerErrorType::UnknownIdentifier(ident) => format!("UnknownIdentifier '{}'", ident),
            _ => self.error_type.to_string(),
        };
        write!(f, "{}: {}", self.location, type_str)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct FileLocation {
    pub line: usize,
    pub column: usize,
}

impl std::fmt::Display for FileLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}, column {}", self.line, self.column)
    }
}

pub struct Lexer<'a> {
    input: &'a [u8],
    position: usize,
    file_location: FileLocation,
    previous_token_location: FileLocation,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        Self {
            input,
            position: 0,
            file_location: FileLocation { line: 1, column: 0 },
            previous_token_location: FileLocation { line: 1, column: 0 },
        }
    }

    pub fn previous_token_location(&self) -> FileLocation {
        self.previous_token_location
    }

    pub fn next_token(&mut self) -> Result<Token, LexerError> {
        // Seek past any whitespace
        loop {
            match self.get_char() {
                Some(c) => match c {
                    ' ' | '\t' | '\n' | '\r' => (),
                    _ => {
                        self.unget_char();
                        break;
                    }
                },
                None => {
                    return Err(LexerError::new(
                        LexerErrorType::EndOfInput,
                        FileLocation {
                            line: self.file_location.line,
                            column: self.file_location.column.saturating_sub(1),
                        },
                    ))
                }
            }
        }

        // Collect the token
        let mut identifier_start_position = None;
        self.previous_token_location = self.file_location;
        loop {
            match self.get_char() {
                Some(c) => match c {
                    '#' => {
                        'comment: loop {
                            match self.get_char() {
                                Some('\n' | '\r') => {
                                    break 'comment;
                                }
                                None => {
                                    return Err(LexerError::new(
                                        LexerErrorType::EndOfInput,
                                        self.previous_token_location,
                                    ));
                                }
                                _ => (),
                            }
                        }
                        return self.next_token();
                    }
                    '"' => {
                        let start_position = self.position;
                        loop {
                            match self.get_char() {
                                Some(c) => match c {
                                    '"' => {
                                        return Ok(Token::String(
                                            unsafe {
                                                std::str::from_utf8_unchecked(
                                                    &self.input[start_position..self.position - 1],
                                                )
                                            }
                                            .into(),
                                        ));
                                    }
                                    '\\' => match self.get_char() {
                                        Some(_) => (),
                                        None => {
                                            return Err(LexerError::new(
                                                LexerErrorType::UnexpectedEndOfInput,
                                                FileLocation {
                                                    line: self.file_location.line,
                                                    column: self.file_location.column - 1,
                                                },
                                            ));
                                        }
                                    },
                                    '\n' => {
                                        return Err(LexerError::new(
                                            LexerErrorType::UnterminatedString,
                                            FileLocation {
                                                line: self.file_location.line,
                                                column: self.file_location.column - 1,
                                            },
                                        ))
                                    }
                                    _ => (),
                                },
                                None => {
                                    return Err(LexerError::new(
                                        LexerErrorType::UnexpectedEndOfInput,
                                        FileLocation {
                                            line: self.file_location.line,
                                            column: self.file_location.column - 1,
                                        },
                                    ));
                                }
                            }
                        }
                    }
                    '[' => {
                        return Ok(Token::LeftBracket);
                    }
                    ' ' | '\t' | '\r' | '\n' | ']' => {
                        if let Some(position) = identifier_start_position.take() {
                            let end_position = self.position - 1;

                            // We need to return right bracket as a the next token if encountered
                            if c == ']' {
                                self.position -= 1;
                            }

                            return Token::try_from(&self.input[position..end_position]).map_err(
                                |err| match err {
                                    TokenError::InvalidNumber => LexerError::new(
                                        LexerErrorType::InvalidNumber,
                                        self.previous_token_location,
                                    ),
                                    TokenError::UnknownIdentifier(ident) => LexerError::new(
                                        LexerErrorType::UnknownIdentifier(ident.to_string()),
                                        self.previous_token_location,
                                    ),
                                },
                            );
                        } else if c == ']' {
                            return Ok(Token::RightBracket);
                        }
                    }
                    _ => {
                        if identifier_start_position.is_none() {
                            identifier_start_position = Some(self.position - 1);
                            self.previous_token_location = FileLocation {
                                line: self.file_location.line,
                                column: self.file_location.column - 1,
                            }
                        }
                    }
                },
                None => {
                    return Err(LexerError::new(
                        LexerErrorType::EndOfInput,
                        FileLocation {
                            line: self.file_location.line,
                            column: self.file_location.column - 1,
                        },
                    ))
                }
            }
        }
    }

    fn get_char(&mut self) -> Option<char> {
        if self.position < self.input.len() {
            let c = self.input[self.position] as char;
            self.position += 1;

            if c == '\n' {
                self.file_location.line += 1;
                self.file_location.column = 0;
            } else {
                self.file_location.column += 1;
            }

            Some(c)
        } else {
            None
        }
    }

    fn unget_char(&mut self) {
        debug_assert!(
            self.file_location.column > 0,
            "Tried to move lexer back past a newline"
        );

        self.position -= 1;
        self.file_location.column -= 1;
    }
}

#[derive(Debug)]
pub enum Token {
    Number(f64),
    // TODO: Check how much this being owned affects perf.
    //       Having Token keep a ref into Lexer makes parser control flow hairy.
    String(String),
    LeftBracket,
    RightBracket,
    Accelerator,
    ActiveTransform,
    All,
    AreaLightSource,
    AttributeBegin,
    AttributeEnd,
    Camera,
    ConcatTransform,
    CoordinateSystem,
    CoordSysTransform,
    EndTime,
    Film,
    Identity,
    Include,
    LightSource,
    LookAt,
    MakeNamedMaterial,
    MakeNamedMedium,
    Material,
    MediumInterface,
    NamedMaterial,
    ObjectBegin,
    ObjectEnd,
    ObjectInstance,
    PixelFilter,
    ReverseOrientation,
    Rotate,
    Sampler,
    Scale,
    Shape,
    StartTime,
    Integrator,
    Texture,
    Transform,
    TransformBegin,
    TransformEnd,
    TransformTimes,
    Translate,
    WorldBegin,
    WorldEnd,
}

pub enum TokenError<'a> {
    InvalidNumber,
    UnknownIdentifier(&'a str),
}

impl<'a> TryFrom<&'a [u8]> for Token {
    type Error = TokenError<'a>;

    fn try_from(t: &'a [u8]) -> Result<Token, TokenError<'a>> {
        Ok(match unsafe { std::str::from_utf8_unchecked(t) } {
            "Accelerator" => Token::Accelerator,
            "ActiveTransform" => Token::ActiveTransform,
            "All" => Token::All,
            "AreaLightSource" => Token::AreaLightSource,
            "AttributeBegin" => Token::AttributeBegin,
            "AttributeEnd" => Token::AttributeEnd,
            "Camera" => Token::Camera,
            "ConcatTransform" => Token::ConcatTransform,
            "CoordinateSystem" => Token::CoordinateSystem,
            "CoordSysTransform" => Token::CoordSysTransform,
            "EndTime" => Token::EndTime,
            "Film" => Token::Film,
            "Identity" => Token::Identity,
            "Include" => Token::Include,
            "Integrator" => Token::Integrator,
            "LightSource" => Token::LightSource,
            "LookAt" => Token::LookAt,
            "MakeNamedMedium" => Token::MakeNamedMedium,
            "MakeNamedMaterial" => Token::MakeNamedMaterial,
            "Material" => Token::Material,
            "MediumInterface" => Token::MediumInterface,
            "NamedMaterial" => Token::NamedMaterial,
            "ObjectBegin" => Token::ObjectBegin,
            "ObjectEnd" => Token::ObjectEnd,
            "ObjectInstance" => Token::ObjectInstance,
            "PixelFilter" => Token::PixelFilter,
            "ReverseOrientation" => Token::ReverseOrientation,
            "Rotate" => Token::Rotate,
            "Sampler" => Token::Sampler,
            "Scale" => Token::Scale,
            "Shape" => Token::Shape,
            "StartTime" => Token::StartTime,
            "Texture" => Token::Texture,
            "TransformBegin" => Token::TransformBegin,
            "TransformEnd" => Token::TransformEnd,
            "TransformTimes" => Token::TransformTimes,
            "Transform" => Token::Transform,
            "Translate" => Token::Translate,
            "WorldBegin" => Token::WorldBegin,
            "WorldEnd" => Token::WorldEnd,
            t_str => match t[0] as char {
                '-' | '.' | '0'..='9' => match t_str.parse::<f64>() {
                    Ok(v) => Token::Number(v),
                    Err(_) => {
                        return Err(TokenError::InvalidNumber);
                    }
                },
                _ => {
                    return Err(TokenError::UnknownIdentifier(t_str));
                }
            },
        })
    }
}
