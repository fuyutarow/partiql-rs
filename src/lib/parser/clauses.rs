use nom::branch::alt;
use nom::bytes::complete::{tag, tag_no_case};
use nom::character::complete::multispace0;
use nom::combinator::opt;
use nom::error::context;
use nom::multi::separated_list1;
use nom::sequence::{preceded, tuple};
use nom::IResult;

use crate::sql::Field;
use crate::sql::Proj;
use crate::sql::WhereCond;

pub use crate::parser::elements;
pub use crate::parser::elements::comma;
pub use crate::parser::expressions;
pub use crate::parser::keywords;
pub use crate::parser::parse_expr;
pub use crate::parser::parse_value;
pub use crate::parser::string_allowed_in_field;
pub use crate::sql::clause;

pub fn select(input: &str) -> IResult<&str, Vec<Proj>> {
    let (input, vec) = context(
        "select claues",
        preceded(
            tag_no_case("SELECT"),
            preceded(multispace0, separated_list1(comma, expressions::parse_proj)),
        ),
    )(input)?;
    Ok((input, vec))
}

pub fn from<'a>(input: &'a str) -> IResult<&'a str, Vec<Field>> {
    let (input, fields) = context(
        "from clause",
        preceded(
            tag_no_case("FROM"),
            preceded(
                multispace0,
                preceded(
                    multispace0,
                    separated_list1(comma, expressions::parse_field),
                ),
            ),
        ),
    )(input)?;
    Ok((input, fields))
}

pub fn left_join<'a>(input: &'a str) -> IResult<&'a str, Vec<Field>> {
    let (input, fields) = context(
        "left join clause",
        preceded(
            tag_no_case("LEFT JOIN"),
            preceded(
                multispace0,
                preceded(
                    multispace0,
                    separated_list1(comma, expressions::parse_field),
                ),
            ),
        ),
    )(input)?;
    Ok((input, fields))
}

pub fn parse_where(input: &str) -> IResult<&str, WhereCond> {
    preceded(
        tag_no_case("WHERE"),
        alt((parse_where_eq, parse_where_like)),
    )(input)
}

pub fn parse_where_eq(input: &str) -> IResult<&str, WhereCond> {
    let (input, (expr, _, right)) = preceded(
        multispace0,
        tuple((
            parse_expr,
            preceded(multispace0, tag("=")),
            preceded(multispace0, parse_value),
        )),
    )(input)?;
    let res = WhereCond::Eq { expr, right };
    Ok((input, res))
}

pub fn parse_where_like(input: &str) -> IResult<&str, WhereCond> {
    let (input, (expr, _, s)) = preceded(
        multispace0,
        tuple((
            parse_expr,
            preceded(multispace0, tag_no_case("LIKE")),
            preceded(multispace0, elements::string),
        )),
    )(input)?;
    let res = WhereCond::Like {
        expr,
        right: s.to_string(),
    };
    Ok((input, res))
}

pub fn orderby(input: &str) -> IResult<&str, clause::OrderBy> {
    let (input, (_, field_name, opt_asc_or_desc)) = tuple((
        tag_no_case("ORDER BY"),
        preceded(multispace0, string_allowed_in_field),
        preceded(
            multispace0,
            opt(alt((tag_no_case("ASC"), tag_no_case("DESC")))),
        ),
    ))(input)?;

    let is_asc = opt_asc_or_desc
        .map(|asc_or_desc| asc_or_desc.to_lowercase() == "asc")
        .unwrap_or(true);
    Ok((
        input,
        clause::OrderBy {
            label: field_name,
            is_asc,
        },
    ))
}

pub fn limit(input: &str) -> IResult<&str, clause::Limit> {
    let (input, (_, limit, opt_offset)) = tuple((
        tag_no_case("LIMIT"),
        preceded(multispace0, elements::integer),
        opt(preceded(multispace0, offset)),
    ))(input)?;

    let offset = opt_offset.unwrap_or(0);
    Ok((input, clause::Limit { limit, offset }))
}

pub fn offset(input: &str) -> IResult<&str, u64> {
    preceded(
        tag_no_case("OFFSET"),
        preceded(multispace0, elements::integer),
    )(input)
}
