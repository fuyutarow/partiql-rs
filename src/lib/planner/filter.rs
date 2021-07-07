use crate::sql::re_from_str;
use crate::sql::Env;
use crate::sql::Expr;
use crate::sql::Selector;
use crate::sql::WhereCond;
use crate::value::PqlValue;

#[derive(Debug, Default, Clone)]
pub struct Filter(pub Option<Box<WhereCond>>);

impl Filter {
    pub fn execute(self, value: PqlValue, env: &Env) -> PqlValue {
        match &self.0 {
            None => value,
            Some(box WhereCond::Eq { expr, right }) => match expr {
                Expr::Selector(selector) => {
                    let selector = selector.expand_fullpath2(&env);
                    let cond = WhereCond::Eq {
                        expr: expr.to_owned(),
                        right: right.to_owned(),
                    };
                    value
                        .restrict(&selector, &Some(cond))
                        .expect("restricted value")
                }
                _ => {
                    todo!();
                }
            },
            Some(box WhereCond::Like { expr, right }) => match expr {
                Expr::Selector(selector) => {
                    let selector = selector.expand_fullpath2(&env);
                    let cond = WhereCond::Like {
                        expr: expr.to_owned(),
                        right: right.to_owned(),
                    };
                    value
                        .restrict(&selector, &Some(cond))
                        .expect("restricted value")
                }
                _ => {
                    todo!();
                }
            },
            _ => {
                dbg!(&self);
                todo!()
            }
        }
    }
}

pub fn restrict(
    value: Option<PqlValue>,
    path: &Selector,
    cond: &Option<WhereCond>,
) -> Option<PqlValue> {
    match value {
        None => None,
        Some(PqlValue::Boolean(boolean)) if boolean => Some(PqlValue::Boolean(boolean)),
        Some(PqlValue::Boolean(_)) => None,
        Some(PqlValue::Null) => None,
        Some(PqlValue::Str(string)) => {
            let is_match = match cond {
                Some(WhereCond::Eq { expr: _, right }) => {
                    PqlValue::Str(string.clone()) == right.to_owned()
                }
                Some(WhereCond::Like { expr: _, right }) => re_from_str(&right).is_match(&string),

                _ => unreachable!(),
            };
            if is_match {
                Some(PqlValue::Str(string.to_owned()))
            } else {
                None
            }
        }
        Some(PqlValue::Float(float)) => Some(PqlValue::Float(float)),
        Some(PqlValue::Array(array)) => {
            let arr = array
                .into_iter()
                .filter_map(|v| {
                    let vv = restrict(Some(v), path, cond);
                    vv
                })
                .collect::<Vec<_>>();

            if arr.is_empty() {
                None
            } else {
                Some(PqlValue::Array(arr))
            }
        }
        Some(PqlValue::Object(mut object)) => {
            if let Some((head, tail)) = &path.split_first() {
                if let Some(value) = object.get(&head.to_string()) {
                    match restrict(Some(value.to_owned()), &tail, cond) {
                        Some(v) if tail.to_vec().len() > 0 => {
                            let it = object.get_mut(&head.to_string()).unwrap();
                            *it = v.to_owned();
                            Some(PqlValue::Object(object))
                        }
                        Some(_v) => Some(PqlValue::Object(object.to_owned())),
                        _ => None,
                    }
                } else {
                    None
                }
            } else {
                unreachable!()
            }
        }
        _ => {
            todo!();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::restrict;
    use crate::pqlir_parser;
    use crate::sql::Expr;
    use crate::sql::Selector;
    use crate::sql::WhereCond;
    use crate::value::PqlValue;

    #[test]
    fn boolean() -> anyhow::Result<()> {
        let value = PqlValue::from_str(
            "
    <<true, false, null>>
   ",
        )?;

        let res = value.restrict(&Selector::default(), &None);
        assert_eq!(res, Some(PqlValue::from_str(r#"<<true>>"#)?));
        Ok(())
    }

    #[test]
    fn missing() -> anyhow::Result<()> {
        let value = PqlValue::from_str(
            "
{
    'top': <<
        {'a': 1, 'b': true, 'c': 'alpha'},
        {'a': 2, 'b': null, 'c': 'beta'},
        {'a': 3, 'c': 'gamma'}
    >>
}
   ",
        )?;
        let res = value.restrict(&Selector::from("top.b"), &None);
        let expected = pqlir_parser::pql_value(
            "
{
    'top': <<
        {'a': 1, 'b': true, 'c': 'alpha'}
    >>
}
   ",
        )?;
        assert_eq!(res, Some(expected));

        Ok(())
    }

    #[test]
    fn pattern_string() -> anyhow::Result<()> {
        let value = PqlValue::from_str(
            "
{
    'hr': {
        'employeesNest': <<
            {
            'id': 3,
            'name': 'Bob Smith',
            'title': null,
            'projects': [ { 'name': 'AWS Redshift Spectrum querying' },
                            { 'name': 'AWS Redshift security' },
                            { 'name': 'AWS Aurora security' }
                        ]
            },
            {
                'id': 4,
                'name': 'Susan Smith',
                'title': 'Dev Mgr',
                'projects': []
            },
            {
                'id': 6,
                'name': 'Jane Smith',
                'title': 'Software Eng 2',
                'projects': [ { 'name': 'AWS Redshift security' } ]
            }
        >>
    }
}
   ",
        )?;
        let selector = Selector::from("hr.employeesNest.projects.name");
        let cond = WhereCond::Like {
            expr: Expr::default(),
            right: "%security%".to_owned(),
        };
        let res = value.restrict(&selector, &Some(cond));
        let expected = pqlir_parser::pql_value(
            "
{
    'hr': {
        'employeesNest': <<
            {
                'id': 3,
                'name': 'Bob Smith',
                'title': null,
                'projects': [
                    { 'name': 'AWS Redshift security' },
                    { 'name': 'AWS Aurora security' }
                ]
            },
            {
                'id': 6,
                'name': 'Jane Smith',
                'title': 'Software Eng 2',
                'projects': [ { 'name': 'AWS Redshift security' } ]
            }
        >>
    }
}
   ",
        )?;
        assert_eq!(res, Some(expected));

        Ok(())
    }
}
