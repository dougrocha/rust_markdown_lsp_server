use lib_parser::yaml::Yaml;

#[derive(Debug, Clone, PartialEq)]
pub enum FrontmatterValue {
    String(String),
    StringList(Vec<String>),
}

impl FrontmatterValue {
    pub fn as_string(&self) -> Option<&str> {
        match self {
            FrontmatterValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[String]> {
        match self {
            FrontmatterValue::StringList(list) => Some(list),
            _ => None,
        }
    }

    pub fn to_string_list(&self) -> Vec<String> {
        match self {
            FrontmatterValue::String(s) => vec![s.clone()],
            FrontmatterValue::StringList(list) => list.clone(),
        }
    }
}

impl From<Yaml<'_>> for FrontmatterValue {
    fn from(value: Yaml<'_>) -> Self {
        match value {
            Yaml::String(str) => FrontmatterValue::String(str.to_string()),
            Yaml::List(items) => FrontmatterValue::StringList(
                items.into_iter().map(|item| item.to_string()).collect(),
            ),
        }
    }
}
