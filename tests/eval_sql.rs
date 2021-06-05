use indexmap::IndexMap;

use itertools::Itertools;

use partiql::parser;
use partiql::pqlir_parser;
use partiql::sql;
use partiql::sql::evaluate;
use partiql::sql::Sql;
use partiql::value::PqlValue;

fn get_sql_data_output(qi: &str) -> anyhow::Result<(Sql, PqlValue, PqlValue)> {
    let sql = {
        let input = std::fs::read_to_string(format!("samples/{}.sql", qi)).unwrap();
        let sql = parser::sql(&input)?;
        sql
    };

    let data = {
        let input = std::fs::read_to_string(format!("samples/{}.env", qi)).unwrap();
        let model = pqlir_parser::pql_model(&input)?;
        model
    };

    let output = {
        let input = std::fs::read_to_string(format!("samples/{}.output", qi)).unwrap();
        let v = input.split("---").collect::<Vec<_>>();
        let input = v.first().unwrap();
        let model = pqlir_parser::pql_model(&input)?;
        model
    };

    Ok((sql, data, output))
}

#[test]
fn q1() -> anyhow::Result<()> {
    let (sql, data, output) = get_sql_data_output("q1")?;
    let res = evaluate(&sql, &data);
    assert_eq!(res, output);
    Ok(())
}

#[test]
fn q2() -> anyhow::Result<()> {
    let (sql, data, output) = get_sql_data_output("q2")?;
    let res = evaluate(&sql, &data);
    assert_eq!(res, output);
    Ok(())
}

#[test]
fn q3() -> anyhow::Result<()> {
    let (sql, data, output) = get_sql_data_output("q3")?;
    let res = evaluate(&sql, &data);
    assert_eq!(res, output);
    Ok(())
}

// #[test]
// fn q4() -> anyhow::Result<()> {
//     let (sql, data, output) = get_sql_data_output("q4")?;
//     let res = evaluate(&sql, &data);
//     assert_eq!(res, output);
//     Ok(())
// }

#[test]
fn q5() -> anyhow::Result<()> {
    let (sql, data, output) = get_sql_data_output("q5")?;
    let res = evaluate(&sql, &data);
    assert_eq!(res, output);
    Ok(())
}

// #[test]
// fn q6() -> anyhow::Result<()> {
//     let (sql, data, output) = get_sql_data_output("q6")?;
//     let res = evaluate(&sql, &data);
//     assert_eq!(res, output);
//     Ok(())
// }

// #[test]
// fn q7() -> anyhow::Result<()> {
//     let (sql, data, output) = get_sql_data_output("q7")?;
//     let res = evaluate(&sql, &data);
//     assert_eq!(res, output);
//     Ok(())
// }
