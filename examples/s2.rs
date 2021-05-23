use std::collections::HashMap;

use itertools::Itertools;

use partiql::dsql_parser;
use partiql::models::JsonValue;
use partiql::pqlir_parser;
use partiql::sql::to_list;
use partiql::sql::Bingings;
use partiql::sql::DField;
use partiql::sql::DSql;
use partiql::sql::DWhereCond;
use partiql::sql::Dpath;
use partiql::sql_parser;

fn main() {
    parse();
}

fn parse() -> anyhow::Result<()> {
    let sql = {
        let input = std::fs::read_to_string("samples/q2.sql").unwrap();
        let sql = dsql_parser::sql(&input)?;
        sql
    };

    let data = {
        let input = std::fs::read_to_string("samples/q2.env").unwrap();
        let model = pqlir_parser::pql_model(&input)?;
        model
    };

    let fields = sql
        .select_clause
        .iter()
        .chain(sql.from_clause.iter())
        .map(|e| e.to_owned())
        .collect::<Vec<_>>();
    let bindings = Bingings::from(fields.as_slice());

    let select_fields = sql
        .select_clause
        .iter()
        .map(|field| field.to_owned().full(&bindings))
        .collect::<Vec<_>>();
    let bindings_for_select = Bingings::from(select_fields.as_slice());

    let value = data.select_by_fields(&select_fields).unwrap();
    let list = to_list(value);
    let filtered_list = list
        .iter()
        .filter_map(|value| match &sql.where_clause {
            Some(cond) if cond.eval(&value.to_owned(), &bindings, &bindings_for_select) => {
                Some(value.to_owned())
            }
            _ => None,
        })
        .collect::<Vec<JsonValue>>();

    let output = {
        let input = std::fs::read_to_string("samples/q2.output").unwrap();
        let v = input.split("---").collect::<Vec<_>>();
        let input = v.first().unwrap();
        let model = pqlir_parser::pql_model(&input)?;
        model
    };

    assert_eq!(JsonValue::Array(filtered_list), output);

    dbg!("END OF FILE");
    Ok(())
}
