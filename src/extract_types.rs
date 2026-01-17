use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use serde::Deserialize;
use std::mem::take;

#[derive(Deserialize)]
struct ProjectFile {
    tree: Option<Tree>,
}

#[derive(Deserialize)]
struct Tree {
    #[serde(rename = "$path")]
    path: String,
}

fn get(code: &str, at: usize) -> char {
    if at >= code.len() {
        return '\0';
    }
    code.as_bytes()[at] as char
}

fn is_end_of_block(code: &str, at: usize, level: usize) -> bool {
    if get(code, at) != ']' {
        return false;
    }
    let mut index = at + 1;
    for _ in 0..level {
        if get(code, index) != '=' {
            return false;
        }
        index += 1;
    }
    if get(code, index) != ']' {
        return false;
    }
    true
}

#[derive(Clone)]
enum LexState {
    Code,
    TemplateString, // ``
    DoubleQuoteString, // ""
    SingleQuoteString, // ''
    BlockString(usize), // [=[ ]=]
    LineComment, // --
    BlockComment(usize), // --[=[ ]=]
}

fn strip_comments_and_strings(lua_code: &str) -> String {
    // Buffer to write out to charater by character
    let mut output = String::new();
    let mut index = 0;
    let mut state = LexState::Code;
    while index < lua_code.len() {
        let c = get(lua_code, index);
        let peek = get(lua_code, index + 1);
        match (state.clone(), c, peek) {
            // Checks to enter one of the states to ignore
            (LexState::Code, '`', _) => {
                state = LexState::TemplateString;
                index += 1;
            }
            (LexState::Code, '"', _) => {
                state = LexState::DoubleQuoteString;
                index += 1;
            }
            (LexState::Code, '\'', _) => {
                state = LexState::SingleQuoteString;
                index += 1;
            }
            (LexState::Code, '[', '=' | '[') => {
                index += 2;
                let mut level = if peek == '=' { 1 } else { 0 };
                while get(lua_code, index) == '=' {
                    level += 1;
                    index += 1;
                }
                state = LexState::BlockString(level);
                if level > 0 {
                    assert!(get(lua_code, index) == '[');
                    index += 1;
                }
            }
            (LexState::Code, '-', '-') => {
                index += 2;
                if get(lua_code, index) == '[' {
                    index += 1;
                    let mut level = 0;
                    while get(lua_code, index) == '=' {
                        level += 1;
                        index += 1;
                    }
                    if get(lua_code, index) == '[' {
                        index += 1;
                        state = LexState::BlockComment(level);
                    } else {
                        state = LexState::LineComment;
                    }
                } else {
                    state = LexState::LineComment;
                }
            }

            // Useful contents to not ignore
            (LexState::Code, _, _) => {
                output.push(c);
                index += 1;
            }

            // Template string
            (LexState::TemplateString, '`', _) => {
                state = LexState::Code;
                index += 1;
            }
            (LexState::TemplateString, '\\', '`') => {
                index += 2;
            }

            // Double quote string
            (LexState::DoubleQuoteString, '"', _) => {
                state = LexState::Code;
                index += 1;
            }
            (LexState::DoubleQuoteString, '\\', '"') => {
                index += 2;
            }

            // Single quote string
            (LexState::SingleQuoteString, '\'', _) => {
                state = LexState::Code;
                index += 1;
            }
            (LexState::SingleQuoteString, '\\', '\'') => {
                index += 2;
            }

            // Block string
            (LexState::BlockString(depth), ']', '=' | ']') => {
                if is_end_of_block(lua_code, index, depth) {
                    state = LexState::Code;
                    index += depth + 2;
                } else {
                    index += 1;
                }
            }

            // Block comment
            (LexState::BlockComment(depth), ']', '=' | ']') => {
                if is_end_of_block(lua_code, index, depth) {
                    state = LexState::Code;
                    index += depth + 2;
                } else {
                    index += 1;
                }
            }

            // Line comment
            (LexState::LineComment, '\n', _) => {
                state = LexState::Code;
            }

            // Other character to ignore
            (_, _, _) => {
                index += 1;
            }
        }
    }

    output
}

#[derive(Clone, PartialEq, Debug)]
enum ParseState {
    Code, // expect "export"
    Export, // expect "type"
    Type, // expect type name
    StartTypeParamList, // optionally expect '<'
    TypeParam, // expect type param name
    TypePack, // optionally expect "..."
    TypeDefault, // optionally expect '=' and default value
    TypeDefaultName, // expect default type name
    NextTypeParam, // optionally expect ',' or '>'
}

#[derive(Clone)]
pub struct TypeParam {
    name: String,
    is_pack: bool,
    default: Option<String>,
}

impl TypeParam {
    fn new() -> Self {
        TypeParam {
            name: String::new(),
            is_pack: false,
            default: None,
        }
    }
}

impl Default for TypeParam {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ExportStatement {
    name: String,
    is_exported: bool,
    type_params: Vec<TypeParam>,
}

impl ExportStatement {
    fn new() -> Self {
        ExportStatement {
            name: String::new(),
            is_exported: false,
            type_params: Vec::new(),
        }
    }

    pub fn to_forwarding_statement(&self, module_name: &str) -> String {
        if self.type_params.len() == 0 {
            format!("export type {} = {}.{}", self.name, module_name, self.name)
        } else {
            let params: Vec<String> = self.type_params.iter().map(|param| {
                let pack = if param.is_pack { "..." } else { "" };
                let default = param.default.as_ref().map(|d| format!(" = {}", d)).unwrap_or_default();
                format!("{}{}{}", param.name, pack, default)
            }).collect();

            let param_names: Vec<String> = self.type_params.iter().map(|param| {
                let pack = if param.is_pack { "..." } else { "" };
                format!("{}{}", param.name, pack)
            }).collect();

            format!(
                "export type {}<{}> = {}.{}<{}>",
                self.name,
                params.join(", "),
                module_name,
                self.name,
                param_names.join(", ")
            )
        }
    }
}

impl Default for ExportStatement {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ExtractTypesResult {
    statements: Vec<ExportStatement>,
}

impl ExtractTypesResult {
    pub fn new() -> Self {
        ExtractTypesResult {
            statements: Vec::new(),
        }
    }

    pub fn format_forwarding_statements(&self, module_name: &str) -> String {
        self.statements.iter().map(|stmt| {
            stmt.to_forwarding_statement(module_name)
        }).collect::<Vec<String>>().join("\n")
    }

    pub fn is_empty(&self) -> bool {
        self.statements.is_empty()
    }

    pub fn add_statement(&mut self, statement: ExportStatement) {
        if statement.is_exported {
            self.statements.push(statement);
        }
    }
}

fn parse_types(lua_code: &str) -> ExtractTypesResult {
    // First strip any comments / strings which could have extraneous "export type" text in them.
    let lua_code = strip_comments_and_strings(lua_code);

    // Now use a permissive parse to find export type statements.
    let mut index = 0;
    let mut state = ParseState::Code;
    let mut current_export_statement = ExportStatement::new();
    let mut current_type_param = TypeParam::new();
    let mut result = ExtractTypesResult::new();
    let mut non_exported_types: BTreeSet<String> = BTreeSet::new();
    while index < lua_code.len() {
        let mut c = get(&lua_code, index);
        // Skip whitespace
        while c.is_ascii_whitespace() {
            index += 1;
            c = get(&lua_code, index);
        }
        if index >= lua_code.len() {
            break;
        }
        match (state.clone(), c) {
            (ParseState::Code, 'e') => {
                if lua_code[index..].starts_with("export") {
                    state = ParseState::Export;
                    current_export_statement.is_exported = true;
                    index += "export".len();
                } else {
                    index += 1;
                }
            }
            (ParseState::Code, 't') => {
                if lua_code[index..].starts_with("type") {
                    state = ParseState::Type;
                    current_export_statement.is_exported = false;
                    index += "type".len();
                } else {
                    index += 1;
                }
            }
            (ParseState::Export, 't') => {
                if lua_code[index..].starts_with("type") {
                    state = ParseState::Type;
                    index += "type".len();
                } else {
                    state = ParseState::Code;
                }
            }
            (ParseState::Type, _) => {
                let start = index;
                while get(&lua_code, index).is_ascii_alphanumeric() || get(&lua_code, index) == '_' {
                    index += 1;
                }
                let type_name = &lua_code[start..index];
                current_export_statement.name = type_name.to_string();
                if !current_export_statement.is_exported {
                    non_exported_types.insert(type_name.to_string());
                }
                state = ParseState::StartTypeParamList;
            }
            (ParseState::StartTypeParamList, '<') => {
                state = ParseState::TypeParam;
                index += 1;
            }
            (ParseState::StartTypeParamList, _) => {
                result.add_statement(take(&mut current_export_statement));
                state = ParseState::Code;
            }
            (ParseState::TypeParam, _) => {
                let start = index;
                while get(&lua_code, index).is_ascii_alphanumeric() || get(&lua_code, index) == '_' {
                    index += 1;
                }
                let param_name = &lua_code[start..index];
                assert!(param_name.len() > 0);
                current_type_param.name = param_name.to_string();
                state = ParseState::TypePack;
            }
            (ParseState::TypePack, '.') => {
                if lua_code[index..].starts_with("...") {
                    current_type_param.is_pack = true;
                    index += 3;
                }
                state = ParseState::TypeDefault;
            }
            (ParseState::TypePack, _) => {
                state = ParseState::TypeDefault;
            }
            (ParseState::TypeDefault, '=') => {
                index += 1;
                state = ParseState::TypeDefaultName;
            }
            (ParseState::TypeDefault, _) => {
                current_export_statement.type_params.push(take(&mut current_type_param));
                state = ParseState::NextTypeParam;
            }
            (ParseState::TypeDefaultName, _) => {
                let start = index;
                while get(&lua_code, index).is_ascii_alphanumeric() || get(&lua_code, index) == '_' {
                    index += 1;
                }
                let default_name = &lua_code[start..index];
                assert!(default_name.len() > 0);
                current_type_param.default = Some(default_name.to_string());
                current_export_statement.type_params.push(take(&mut current_type_param));
                state = ParseState::NextTypeParam;
            }
            (ParseState::NextTypeParam, ',') => {
                index += 1;
                state = ParseState::TypeParam;
            }
            (ParseState::NextTypeParam, '>') => {
                result.add_statement(take(&mut current_export_statement));
                index += 1;
                state = ParseState::Code;
            }
            _ => {
                index += 1;
            }
        }
    }

    // Post-process to remove type defaults which weren't exported.
    // There's no way to reference these types from outside the module so there's
    // no way to re-export them. The library author has to fix this if desired.
    for statement in result.statements.iter_mut() {
        for param in statement.type_params.iter_mut() {
            if let Some(default) = &param.default {
                if non_exported_types.contains(default) {
                    param.default = None;
                }
            }
        }
    }

    result
}

pub fn extract_types(package_path: &PathBuf) -> ExtractTypesResult {
    log::debug!("Processing types for package at {}", package_path.display());

    let project_file_path = package_path.join("default.project.json");

    if !project_file_path.exists() {
        log::debug!("No default.project.json found for package at {}", package_path.display());
        return ExtractTypesResult::new();
    }

    let project_contents = match fs::read_to_string(&project_file_path) {
        Ok(c) => c,
        Err(err) => {
            log::warn!(
                "Failed to read {}: {}",
                project_file_path.display(),
                err
            );
            return ExtractTypesResult::new();
        }
    };

    let project: ProjectFile = match serde_json::from_str(&project_contents) {
        Ok(p) => p,
        Err(err) => {
            log::warn!(
                "Invalid JSON in {}: {}",
                project_file_path.display(),
                err
            );
            return ExtractTypesResult::new();
        }
    };

    let tree_path = match project.tree {
        Some(tree) => package_path.join(tree.path),
        None => {
            log::debug!("default.project.json has no tree path");
            return ExtractTypesResult::new();
        }
    };

    let init_lua = tree_path.join("init.lua");
    let init_luau = tree_path.join("init.luau");

    let init_path = if init_lua.exists() {
        init_lua
    } else if init_luau.exists() {
        init_luau
    } else {
        log::debug!(
            "No init.lua or init.luau found under {}",
            tree_path.display()
        );
        return ExtractTypesResult::new();
    };

    let init_contents = match fs::read_to_string(&init_path) {
        Ok(c) => c,
        Err(err) => {
            log::warn!(
                "Failed to read {}: {}",
                init_path.display(),
                err
            );
            return ExtractTypesResult::new();
        }
    };

    parse_types(&init_contents)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_line_comment() {
        let input = "code -- comment\nmore code";
        let expected = "code \nmore code";
        assert_eq!(strip_comments_and_strings(input), expected);
    }

    #[test]
    fn test_strip_block_comment() {
        let input = "code --[[ block comment ]] more";
        let expected = "code  more";
        assert_eq!(strip_comments_and_strings(input), expected);
    }

    #[test]
    fn test_strip_block_comment_with_equals() {
        let input = "code --[=[ block comment ]=] more";
        let expected = "code  more";
        assert_eq!(strip_comments_and_strings(input), expected);
    }

    #[test]
    fn test_strip_double_quote_string() {
        let input = r#"code "export type Foo" more"#;
        let expected = "code  more";
        assert_eq!(strip_comments_and_strings(input), expected);
    }

    #[test]
    fn test_strip_single_quote_string() {
        let input = r#"code 'export type Foo' more"#;
        let expected = "code  more";
        assert_eq!(strip_comments_and_strings(input), expected);
    }

    #[test]
    fn test_strip_template_string() {
        let input = "code `export type Foo` more";
        let expected = "code  more";
        assert_eq!(strip_comments_and_strings(input), expected);
    }

    #[test]
    fn test_strip_block_string() {
        let input = "code [[ block string ]] more";
        let expected = "code  more";
        assert_eq!(strip_comments_and_strings(input), expected);
    }

    #[test]
    fn test_strip_block_string_with_equals() {
        let input = "code [=[ block string ]=] more";
        let expected = "code  more";
        assert_eq!(strip_comments_and_strings(input), expected);
    }

    #[test]
    fn test_strip_escaped_quotes() {
        let input = r#"code "escaped \" quote" more"#;
        let expected = "code  more";
        assert_eq!(strip_comments_and_strings(input), expected);
    }

    #[test]
    fn test_parse_simple_export_type() {
        let input = "export type Foo = string";
        let result = parse_types(input);
        assert_eq!(result.statements.len(), 1);
        assert_eq!(result.statements[0].name, "Foo");
        assert!(result.statements[0].is_exported);
        assert_eq!(result.statements[0].type_params.len(), 0);
    }

    #[test]
    fn test_parse_export_type_with_params() {
        let input = "export type Foo<T, U> = Bar<T, U>";
        let result = parse_types(input);
        assert_eq!(result.statements.len(), 1);
        assert_eq!(result.statements[0].name, "Foo");
        assert_eq!(result.statements[0].type_params.len(), 2);
        assert_eq!(result.statements[0].type_params[0].name, "T");
        assert_eq!(result.statements[0].type_params[1].name, "U");
        assert!(!result.statements[0].type_params[0].is_pack);
        assert!(!result.statements[0].type_params[1].is_pack);
    }

    #[test]
    fn test_parse_export_type_with_parameter_pack() {
        let input = "export type Foo<T...> = Bar<T...>";
        let result = parse_types(input);
        assert_eq!(result.statements.len(), 1);
        assert_eq!(result.statements[0].type_params.len(), 1);
        assert_eq!(result.statements[0].type_params[0].name, "T");
        assert!(result.statements[0].type_params[0].is_pack);
    }

    #[test]
    fn test_parse_export_type_with_default() {
        let input = "export type Foo<T = string> = Bar<T>";
        let result = parse_types(input);
        assert_eq!(result.statements.len(), 1);
        assert_eq!(result.statements[0].type_params.len(), 1);
        assert_eq!(result.statements[0].type_params[0].name, "T");
        assert_eq!(result.statements[0].type_params[0].default, Some("string".to_string()));
    }

    #[test]
    fn test_parse_non_exported_type() {
        let input = "type Foo = string";
        let result = parse_types(input);
        assert_eq!(result.statements.len(), 0);
    }

    #[test]
    fn test_parse_multiple_types() {
        let input = "export type Foo = string\nexport type Bar = number";
        let result = parse_types(input);
        assert_eq!(result.statements.len(), 2);
        assert_eq!(result.statements[0].name, "Foo");
        assert_eq!(result.statements[1].name, "Bar");
    }

    #[test]
    fn test_parse_with_comments() {
        let input = "-- comment\nexport type Foo = string -- inline comment";
        let result = parse_types(input);
        assert_eq!(result.statements.len(), 1);
        assert_eq!(result.statements[0].name, "Foo");
    }

    #[test]
    fn test_parse_ignores_type_in_string() {
        let input = r#"local x = "export type Fake = string"
export type Real = number"#;
        let result = parse_types(input);
        assert_eq!(result.statements.len(), 1);
        assert_eq!(result.statements[0].name, "Real");
    }

    #[test]
    fn test_forwarding_statement_simple() {
        let mut stmt = ExportStatement::new();
        stmt.name = "Foo".to_string();
        stmt.is_exported = true;
        let forwarding = stmt.to_forwarding_statement("Module");
        assert_eq!(forwarding, "export type Foo = Module.Foo");
    }

    #[test]
    fn test_forwarding_statement_with_params() {
        let mut stmt = ExportStatement::new();
        stmt.name = "Foo".to_string();
        stmt.is_exported = true;
        stmt.type_params.push(TypeParam {
            name: "T".to_string(),
            is_pack: false,
            default: None,
        });
        stmt.type_params.push(TypeParam {
            name: "U".to_string(),
            is_pack: false,
            default: Some("string".to_string()),
        });
        let forwarding = stmt.to_forwarding_statement("Module");
        assert_eq!(forwarding, "export type Foo<T, U = string> = Module.Foo<T, U>");
    }

    #[test]
    fn test_forwarding_statement_with_parameter_pack() {
        let mut stmt = ExportStatement::new();
        stmt.name = "Foo".to_string();
        stmt.is_exported = true;
        stmt.type_params.push(TypeParam {
            name: "T".to_string(),
            is_pack: true,
            default: None,
        });
        let forwarding = stmt.to_forwarding_statement("Module");
        assert_eq!(forwarding, "export type Foo<T...> = Module.Foo<T...>");
    }

    #[test]
    fn test_format_forwarding_statements() {
        let mut result = ExtractTypesResult::new();
        let mut stmt1 = ExportStatement::new();
        stmt1.name = "Foo".to_string();
        stmt1.is_exported = true;
        let mut stmt2 = ExportStatement::new();
        stmt2.name = "Bar".to_string();
        stmt2.is_exported = true;
        result.add_statement(stmt1);
        result.add_statement(stmt2);
        
        let output = result.format_forwarding_statements("Module");
        assert_eq!(output, "export type Foo = Module.Foo\nexport type Bar = Module.Bar");
    }

    #[test]
    fn test_remove_non_exported_defaults() {
        let input = r#"
type LocalType = string
export type Foo<T = LocalType> = Bar<T>
"#;
        let result = parse_types(input);
        assert_eq!(result.statements.len(), 1);
        assert_eq!(result.statements[0].type_params[0].default, None);
    }

    #[test]
    fn test_keep_exported_defaults() {
        let input = r#"
export type ExportedType = string
export type Foo<T = ExportedType> = Bar<T>
"#;
        let result = parse_types(input);
        assert_eq!(result.statements.len(), 2);
        assert_eq!(result.statements[1].type_params[0].default, Some("ExportedType".to_string()));
    }

    #[test]
    fn test_complex_type_params() {
        let input = "export type Foo<T, U..., V = string, W... = number> = Bar";
        let result = parse_types(input);
        assert_eq!(result.statements.len(), 1);
        assert_eq!(result.statements[0].type_params.len(), 4);
        
        assert_eq!(result.statements[0].type_params[0].name, "T");
        assert!(!result.statements[0].type_params[0].is_pack);
        assert_eq!(result.statements[0].type_params[0].default, None);
        
        assert_eq!(result.statements[0].type_params[1].name, "U");
        assert!(result.statements[0].type_params[1].is_pack);
        assert_eq!(result.statements[0].type_params[1].default, None);
        
        assert_eq!(result.statements[0].type_params[2].name, "V");
        assert!(!result.statements[0].type_params[2].is_pack);
        assert_eq!(result.statements[0].type_params[2].default, Some("string".to_string()));
        
        assert_eq!(result.statements[0].type_params[3].name, "W");
        assert!(result.statements[0].type_params[3].is_pack);
        assert_eq!(result.statements[0].type_params[3].default, Some("number".to_string()));
    }
}