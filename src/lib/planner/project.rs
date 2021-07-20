use collect_mac::collect;
use indexmap::IndexMap as Map;
use itertools::Itertools;

use crate::sql::Env;
use crate::sql::Expr;
use crate::sql::Field;
use crate::sql::Selector;
use crate::value::PqlValue;

#[derive(Debug, Default, Clone)]
pub struct Projection(pub Vec<Field>);

impl Projection {
    pub fn execute(self, env: &Env) -> Vec<PqlValue> {
        let v = self
            .0
            .into_iter()
            .map(|field| {
                let field = field.expand_fullpath(&env);
                let (alias, expr) = field.rename();
                let value = expr.eval(env);
                (alias, value)
            })
            .collect::<Map<String, PqlValue>>();
        let v = Rows::from(PqlValue::Object(v));
        let v = Records::from(v);
        v.into_list()
    }
}

impl PqlValue {
    pub fn project_by_selector(
        &self,
        alias: Option<String>,
        selector: &Selector,
    ) -> (String, Self) {
        let value = self.select_by_selector(&selector);
        let key = alias.clone().unwrap_or({
            let last = selector.to_vec().last().unwrap().to_string();
            last
        });
        (key, value)
    }

    pub fn select_by_fields(&self, field_list: &[Field], env: &Env) -> Option<Self> {
        let mut new_map = Map::<String, Self>::new();

        for field in field_list {
            match &field.expr {
                Expr::Selector(selector) => {
                    let value = self.select_by_selector(&selector);
                    let key = field.alias.clone().unwrap_or({
                        let last = selector.to_vec().last().unwrap().to_string();
                        last
                    });
                    new_map.insert(key, value);
                }
                _ => {
                    let value = field.to_owned().expr.eval(&env);
                    let key = field.alias.clone().unwrap_or_default();
                    new_map.insert(key, value);
                }
            }
        }

        Some(Self::Object(new_map))
    }
}

#[derive(Debug, Default, Clone)]
pub struct Rows {
    data: Map<String, Vec<PqlValue>>,
    size: usize,
    keys: Vec<String>,
}

impl From<PqlValue> for Rows {
    fn from(value: PqlValue) -> Self {
        let mut size = 0;

        let data = match value {
            PqlValue::Object(record) => record
                .into_iter()
                .map(|(key, val)| match val {
                    PqlValue::Array(array) => {
                        if size == 0 {
                            size = array.len();
                        }
                        (key, array)
                    }
                    _ => {
                        size = 1;
                        (key, vec![val])
                    }
                })
                .collect::<Map<String, Vec<PqlValue>>>(),
            _ => {
                dbg!(&value);
                unreachable!()
            }
        };

        let keys = data.keys().map(String::from).collect();
        Self { data, size, keys }
    }
}

impl From<Rows> for PqlValue {
    fn from(records: Rows) -> Self {
        let array = records
            .data
            .into_iter()
            .map(|(k, v)| {
                PqlValue::Object(collect! {
                    as Map<String, PqlValue>:
                    k => PqlValue::Array(v)
                })
            })
            .collect::<Vec<_>>();
        PqlValue::Array(array)
    }
}

#[derive(Debug, Default, Clone)]
pub struct Records(pub Vec<Map<String, Vec<PqlValue>>>);

impl From<Rows> for Records {
    fn from(rows: Rows) -> Self {
        let records = {
            let mut records = Vec::<Map<String, Vec<PqlValue>>>::new();
            for i in 0..rows.size {
                let mut record = Map::<String, Vec<PqlValue>>::new();
                for key in &rows.keys {
                    if let Some(value) = rows.data.get(key.as_str()).unwrap().get(i) {
                        let v: Vec<PqlValue> = value.to_owned().flatten().into();
                        record.insert(key.to_string(), v);
                    } else {
                        dbg!(&record);
                    }
                }
                records.push(record);
            }
            records
        };
        Self(records)
    }
}

impl From<Records> for PqlValue {
    fn from(records: Records) -> Self {
        Self::Array(
            records
                .0
                .into_iter()
                .map(|obj| {
                    Self::Object(
                        obj.into_iter()
                            .map(|(k, v)| (k, Self::Array(v)))
                            .collect::<Map<String, _>>(),
                    )
                })
                .collect::<Vec<_>>(),
        )
    }
}

impl Records {
    pub fn into_list(self) -> Vec<PqlValue> {
        self.0
            .into_iter()
            .map(|record| {
                let record = record
                    .into_iter()
                    .filter_map(|(k, v)| if !v.is_empty() { Some((k, v)) } else { None })
                    .collect::<Map<String, Vec<PqlValue>>>();

                let keys = record.keys();
                let it = record.values().into_iter().multi_cartesian_product();
                it.map(|prod| {
                    let map = keys
                        .clone()
                        .into_iter()
                        .zip(prod.into_iter())
                        .flat_map(|(key, p)| {
                            p.to_owned()
                                .then_if_not_missing()
                                .map(|val| (key.to_owned(), val))
                        })
                        .collect::<Map<String, _>>();
                    let v = PqlValue::Object(map);
                    v
                })
                .collect::<Vec<PqlValue>>()
            })
            .flatten()
            .collect::<Vec<PqlValue>>()
    }
}

#[cfg(test)]
mod tests {
    use super::Records;
    use super::Rows;
    use crate::planner::LogicalPlan;
    use crate::sql::Env;
    use crate::sql::Expr;
    use crate::sql::Selector;
    use crate::sql::Sql;

    use crate::value::PqlValue;
    use indexmap::IndexMap as Map;
    use std::os::raw::c_longlong;
    use std::str::FromStr;

    #[test]
    fn test_convert_coloumnar_to_rowwise() -> anyhow::Result<()> {
        let form0 = PqlValue::from_str(
            r#"
{
  "projectName": [
    [
      "AWS Redshift security",
      "AWS Aurora security"
    ],
    [
      "AWS Redshift security"
    ]
  ],
  "employeeName": [
    "Bob Smith",
    "Jane Smith"
  ]
}
"#,
        )?;
        let form1 = PqlValue::from_str(
            r#"
[
  {
    "projectName": [
      [
        "AWS Redshift security",
        "AWS Aurora security"
      ],
      [
        "AWS Redshift security"
      ]
    ]
  },
  {
    "employeeName": [
      "Bob Smith",
      "Jane Smith"
    ]
  }
]
"#,
        )?;
        let form2 = PqlValue::from_str(
            r#"
[
  {
    "projectName": [
      "AWS Redshift security",
      "AWS Aurora security"
    ],
    "employeeName": [
      "Bob Smith"
    ]
  },
  {
    "projectName": [
      "AWS Redshift security"
    ],
    "employeeName": [
      "Jane Smith"
    ]
  }
]
"#,
        )?;
        let form3 = PqlValue::from_str(
            r#"
[
  {
    "projectName": "AWS Redshift security",
    "employeeName": "Bob Smith"
  },
  {
    "projectName": "AWS Aurora security",
    "employeeName": "Bob Smith"
  },
  {
    "projectName": "AWS Redshift security",
    "employeeName": "Jane Smith"
  }
]
"#,
        )?;

        let rows = Rows::from(form0.to_owned());
        assert_eq!(PqlValue::from(rows.to_owned()), form1);

        let records = Records::from(rows);
        assert_eq!(PqlValue::from(records.to_owned()), form2);

        let list = records.into_list();
        assert_eq!(PqlValue::from(list.to_owned()), form3);

        Ok(())
    }

    #[test]
    fn test_convert_matrix() -> anyhow::Result<()> {
        let form0 = PqlValue::from_str(
            r#"
{
    "id": [
        1,
        3
    ],
    "x": [
        [
            [2, 4],
            [6]
        ],
        [
            [8]
        ]
    ]
}
"#,
        )?;
        let form3 = PqlValue::from_str(
            r#"
[
  {
    "id": 1,
    "x": 2
  },
  {
    "id": 1,
    "x": 4
  },
  {
    "id": 1,
    "x": 6
  },
  {
    "id": 3,
    "x": 8
  }
]
"#,
        )?;

        let rows = Rows::from(form0.to_owned());
        let v = PqlValue::from(rows.to_owned());
        let records = Records::from(rows);
        let list = records.into_list();
        assert_eq!(PqlValue::from(list.to_owned()), form3);

        Ok(())
    }

    #[test]
    fn test_project_a_missing_value() -> anyhow::Result<()> {
        let env = Env::from(PqlValue::from_str(
            r#"
[
    { 'id': 3, 'name': 'Bob Smith' },
    { 'id': 4, 'name': 'Susan Smith', 'title': 'Dev Mgr' },
    { 'id': 6, 'name': 'Jane Smith', 'title': 'Software Eng 2'}
]
"#,
        )?);
        let sql = Sql::from_str(r#"SELECT id, name, title"#)?;
        let logical_plan = LogicalPlan::from(sql);

        let v = logical_plan
            .project
            .0
            .into_iter()
            .map(|field| {
                let field = field.expand_fullpath(&env);
                let (alias, expr) = field.rename();
                let value = expr.eval(&env);
                (alias, value)
            })
            .collect::<Map<String, PqlValue>>();

        let v = Rows::from(PqlValue::Object(v));
        let v = Records::from(v);
        let v = v.into_list();
        let v = PqlValue::from(v);

        assert_eq!(
            PqlValue::from(v),
            PqlValue::from_str(
                r#"
[
    { 'id': 3, 'name': 'Bob Smith' },
    { 'id': 4, 'name': 'Susan Smith', 'title': 'Dev Mgr' },
    { 'id': 6, 'name': 'Jane Smith', 'title': 'Software Eng 2'}
]
                "#,
            )?
        );
        Ok(())
    }

    #[test]
    fn test_project_a_missing_value2() -> anyhow::Result<()> {
        let env = Env::from(PqlValue::from(vec![
            PqlValue::from_str(
                r#"
    { 'id': 3, 'name': 'Bob Smith' }
            "#,
            )?,
            PqlValue::Missing,
            PqlValue::from_str(
                r#"
    { 'id': 6, 'name': 'Jane Smith', 'title': 'Software Eng 2'}
            "#,
            )?,
        ]));
        let sql = Sql::from_str(r#"SELECT id, name, title"#)?;
        let logical_plan = LogicalPlan::from(sql);

        let v = logical_plan
            .project
            .0
            .into_iter()
            .map(|field| {
                let field = field.expand_fullpath(&env);
                let (alias, expr) = field.rename();
                let value = expr.eval(&env);
                (alias, value)
            })
            .collect::<Map<String, PqlValue>>();

        let v = Rows::from(PqlValue::Object(v));
        let v = Records::from(v);
        let v = v.into_list();
        let v = PqlValue::from(v);

        assert_eq!(
            PqlValue::from(v),
            PqlValue::from_str(
                r#"
[
    { 'id': 3, 'name': 'Bob Smith' },
    {},
    { 'id': 6, 'name': 'Jane Smith', 'title': 'Software Eng 2'}
]
                "#,
            )?
        );
        Ok(())
    }

    #[test]
    fn test_subqueries() -> anyhow::Result<()> {
        let mut env = Env::from(PqlValue::from_str(
            r#"
{
  "hr": {
      "employeesNest": [
         {
            "id": 3,
            "name": "Bob Smith",
            "title": null,
            "projects": [
                { "name": "AWS Redshift Spectrum querying" },
                { "name": "AWS Redshift security" },
                { "name": "AWS Aurora security" }
            ]
          },
          {
              "id": 4,
              "name": "Susan Smith",
              "title": "Dev Mgr",
              "projects": []
          },
          {
              "id": 6,
              "name": "Jane Smith",
              "title": "Software Eng 2",
              "projects": [ { "name": "AWS Redshift security" } ]
          }
      ]
    }
}
        "#,
        )?);
        let sql = Sql::from_str(
            r#"
SELECT e.name AS employeeName,
  ( SELECT p
    FROM e.projects AS p
    WHERE p.name LIKE '%querying%'
  ) AS queryProjectsNum
FROM hr.employeesNest AS e
        "#,
        )?;
        let logical_plan = LogicalPlan::from(sql);

        for drain in logical_plan.drains {
            drain.execute(&mut env);
        }

        logical_plan.filter.execute(&mut env);

        let v = logical_plan
            .project
            .0
            .iter()
            .map(|field| {
                let field = field.expand_fullpath(&env);
                let (alias, expr) = field.rename();
                let value = expr.eval(&env);
                (alias, value)
            })
            .collect::<Map<String, PqlValue>>();

        let v = Rows::from(PqlValue::Object(v));
        let v = Records::from(v);
        let v = v.into_list();
        let v = PqlValue::from(v);

        v.print();
        assert_eq!(
            PqlValue::from(v),
            PqlValue::from_str(
                r#"
<<
  {
    'employeeName': 'Bob Smith',
    'queryProjectsNum': {
      'projects': {
        'name': 'AWS Redshift Spectrum querying'
      }
    }
  },
  {
    'employeeName': 'Susan Smith'
  },
  {
    'employeeName': 'Jane Smith'
  }
>>
                "#,
            )?
        );
        Ok(())
    }
}
