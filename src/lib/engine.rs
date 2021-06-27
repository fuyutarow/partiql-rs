use std::any;
use std::str::FromStr;

use crate::lang::{Lang, LangType};
use crate::parser;
use crate::sql;
use crate::value::JsonValue;
use crate::value::PqlValue;

pub fn evaluate(sql: &str, input: &str, from: &str, to: &str) -> anyhow::Result<String> {
    let from_lang_type = LangType::from_str(&from)?;
    let to_lang_type = LangType::from_str(&to)?;
    let mut lang = Lang::from_as(&input, from_lang_type)?;

    let sql = parser::sql(&sql)?;
    let result = sql::evaluate(&sql, &lang.data);
    lang.to = to_lang_type;
    lang.data = result;
    lang.colnames = sql.get_colnames();
    let output = lang.to_string(true)?;

    Ok(output)
}

pub fn loads(input: &str, from: &str) -> anyhow::Result<PqlValue> {
    let from_lang_type = LangType::from_str(&from)?;
    let mut lang = Lang::from_as(&input, from_lang_type)?;
    let value = lang.data;
    Ok(value)
}

pub fn dumps(data: PqlValue, to: &str) -> anyhow::Result<String> {
    let to_lang_type = LangType::from_str(&to)?;
    let mut lang = Lang::default();
    lang.data = data;
    lang.to = to_lang_type;
    let output = lang.to_string(true)?;
    Ok(output)
}

pub fn query_evaluate(data: PqlValue, sql: &str) -> anyhow::Result<PqlValue> {
    let sql = parser::sql(&sql)?;
    let data = PqlValue::from(data);
    let value = sql::evaluate(&sql, &data);
    Ok(value)
}
