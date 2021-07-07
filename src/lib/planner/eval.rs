pub use crate::planner::LogicalPlan;
pub use crate::sql::clause::Limit;
pub use crate::sql::clause::OrderBy;
use crate::sql::Env;
use crate::sql::Sql;
use crate::value::PqlValue;

pub fn evaluate<'a>(sql: Sql, data: PqlValue) -> PqlValue {
    let mut env = Env::default();
    let plan = LogicalPlan::from(sql);
    let result = plan.execute(&mut env);
    result
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::planner::LogicalPlan;
    use crate::sql::Env;
    use crate::sql::Sql;
    use crate::value::PqlValue;

    #[test]
    fn test_rename() -> anyhow::Result<()> {
        let sql = Sql::from_str(
            r#"
SELECT e.id,
       e.name AS employeeName,
       e.title AS title
FROM
    {
        'employees': <<
            { 'id': 3, 'name': 'Bob Smith',   'title': null },
            { 'id': 4, 'name': 'Susan Smith', 'title': 'Dev Mgr' },
            { 'id': 6, 'name': 'Jane Smith',  'title': 'Software Eng 2'}
        >>
    } AS hr,
    hr.employees e
        "#,
        )?;

        let plan = LogicalPlan::from(sql);
        let mut env = Env::default();
        let res = plan.execute(&mut env);

        assert_eq!(
            res,
            PqlValue::from_str(
                r#"
        [
            { 'id': 3, 'employeeName': 'Bob Smith',   'title': null },
            { 'id': 4, 'employeeName': 'Susan Smith', 'title': 'Dev Mgr' },
            { 'id': 6, 'employeeName': 'Jane Smith',  'title': 'Software Eng 2'}
        ]
            "#
            )?
        );

        Ok(())
    }
}
