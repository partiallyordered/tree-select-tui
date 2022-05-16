// Shallow evaluation of nom and combine showed me that the only meaningful difference I was able
// to discern was that nom was more heavily used. I decided to try nom. Maybe one day I'll compare
// it to combine. Briefly looked at pest also, decided without much thought that I didn't want to
// maintain a separate PEG file (although, as I'm writing this, I realise that could have some
// benefit for interoperability).
//
// TODO: support comments?
// - https://github.com/Geal/nom/blob/294ffb3d9e0ade2c3b7ddfff52484b6d643dcce1/doc/nom_recipes.md#comments
// - only support "until-end-of-line" comments- not in-line comments?
// - just strip all comments before parsing?
// - support some sort of "option" syntax, like:
//     systemctl
//       --user
//         [restart|start|stop|reload]
//           email
//           instant-messaging
//           pulseaudio
//       --system
//         [restart|start|stop|reload]
//           iwd
//           docker
//           firewall
//   or is this just getting too fancy?
extern crate nom;
use nom::{
    error::ParseError,
    IResult,
    combinator::consumed,
    character::complete::{line_ending, space0},
    multi::{count, many0, many1},
    sequence::{pair, delimited, preceded, terminated, tuple},
    bytes::complete::{tag, is_not},
};

#[derive(Debug, PartialEq)]
pub struct Node {
    name: String,
    children: Vec<Node>,
}

fn key(input: &str) -> IResult<&str, &str> {
    let (input, (ws, _)) = consumed(many1(pair(is_not(" \r\n\t"), space0)))(input)?;
    Ok((input, ws))
}

fn empty_lines(input: &str) -> IResult<&str, Vec<(&str, &str)>> {
    many0(pair(space0, line_ending))(input)
}

/// A combinator that takes a parser `inner` and produces a parser that also consumes both leading
/// and trailing empty new lines, returning the output of `inner`.
/// Credit: https://github.com/Geal/nom/blob/294ffb3d9e0ade2c3b7ddfff52484b6d643dcce1/doc/nom_recipes.md#wrapper-combinators-that-eat-whitespace-before-and-after-a-parser
fn strip_empty_lines<'a, F: 'a, O, E: ParseError<&'a str>>(inner: F) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
    where
    F: FnMut(&'a str) -> IResult<&'a str, O, E>,
{
    delimited(
        many0(pair(space0, line_ending)),
        inner,
        many0(pair(space0, line_ending)),
    )
}

fn indented_key<'a>(depth: usize) -> impl FnMut(&'a str) -> IResult<&'a str, &str, nom::error::Error<&'a str>> {
    move |input: &'a str| preceded(count(tag("  "), depth), key)(input)
}

fn node<'a>(depth: usize) -> impl FnMut(&'a str) -> IResult<&'a str, Node, nom::error::Error<&'a str>> {
    move |input: &'a str| {
        let (input, (name, children)) = tuple((
            terminated(indented_key(depth), empty_lines),
            node_list(depth + 1),
        ))(input)?;
        Ok((input, Node { name: name.to_string(), children }))
    }
}

fn node_list<'a>(depth: usize) -> impl FnMut(&'a str) -> IResult<&'a str, Vec<Node>, nom::error::Error<&'a str>> {
    // TODO: Here (or in the node or node_list combinator?) we could detect the indent of the first
    // branch child using the `consumed` or `recognize` combinators
    move |input: &'a str| many0(node(depth))(input)
}

pub fn indent_tree_list(input: &str) -> IResult<&str, Vec<Node>> {
    nom::combinator::all_consuming(node_list(0))(input)
}

mod test {
    use super::*;

    #[test]
    fn test_indented_key_incorrect_indent() {
        let input = "  indented";
        assert_eq!(
            indented_key(0)(input),
            Err(nom::Err::Error(nom::error::Error { input, code: nom::error::ErrorKind::IsNot })),
        );
    }

    #[test]
    fn test_node_incorrect_indent_depth() {
        let input = "  indented";
        assert_eq!(
            node(0)(input),
            Err(nom::Err::Error(nom::error::Error { input, code: nom::error::ErrorKind::IsNot })),
        );
    }

    #[test]
    fn test_key_empty_followed_by_newline_disallowed() {
        let input = "\n";
        assert_eq!(
            key(input),
            Err(nom::Err::Error(nom::error::Error { input, code: nom::error::ErrorKind::IsNot }))
        );
    }

    #[test]
    fn test_key_empty_disallowed() {
        let input = "";
        assert_eq!(
            key(input),
            Err(nom::Err::Error(nom::error::Error { input, code: nom::error::ErrorKind::IsNot }))
        );
    }

    #[test]
    fn test_indent_tree_trailing_chars() {
        let input = "  trailing";
        assert_eq!(
            indent_tree_list(input),
            Err(nom::Err::Error(nom::error::Error { input, code: nom::error::ErrorKind::Eof }))
        );
    }

    #[test]
    fn test_indent_tree_list() {
        let input =
r#"systemctl
  --user
    restart
      pulseaudio
  --system
    restart
      iwd"#;
        assert_eq!(indent_tree_list(input), Ok(("", vec![Node {
            name: "systemctl".to_string(),
            children: vec![
                Node {
                    name: "--user".to_string(),
                    children: vec![
                        Node {
                            name: "restart".to_string(),
                            children: vec![
                                Node {
                                    name: "pulseaudio".to_string(),
                                    children: vec![],
                                },
                            ],
                        },
                    ],
                },
                Node {
                    name: "--system".to_string(),
                    children: vec![
                        Node {
                            name: "restart".to_string(),
                            children: vec![
                                Node {
                                    name: "iwd".to_string(),
                                    children: vec![],
                                },
                            ],
                        },
                    ],
                },
                ],
        }])));
    }

    #[test]
    fn test_node_list_leaf_unindented_multiple() {
        let input = "some\nlist\nof\nindented\nnodes";
        assert_eq!(node_list(0)(input), Ok(("", vec![
            Node { name: "some".to_string(),     children: vec![] },
            Node { name: "list".to_string(),     children: vec![] },
            Node { name: "of".to_string(),       children: vec![] },
            Node { name: "indented".to_string(), children: vec![] },
            Node { name: "nodes".to_string(),    children: vec![] },
        ])))
    }

    #[test]
    fn test_node_list_leaf_indented_single() {
        let input = "  some";
        assert_eq!(
            node_list(1)(input),
            Ok(("", vec![Node { name: "some".to_string(), children: vec![] }]))
        );
    }

    #[test]
    fn test_node_list_leaf_unindented_single() {
        let input = "some";
        assert_eq!(
            node_list(0)(input),
            Ok(("", vec![Node { name: "some".to_string(), children: vec![] }]))
        );
    }

    #[test]
    fn test_node_list_leaf_indented_multiple() {
        let input = "  some\n  list\n  of\n  indented\n  nodes";
        assert_eq!(node_list(1)(input), Ok(("", vec![
            Node { name: "some".to_string(),     children: vec![] },
            Node { name: "list".to_string(),     children: vec![] },
            Node { name: "of".to_string(),       children: vec![] },
            Node { name: "indented".to_string(), children: vec![] },
            Node { name: "nodes".to_string(),    children: vec![] },
        ])))
    }

    #[test]
    fn test_node_list_branch() {
        let input =
r#"  some
    list
  of
    indented
  branch
    nodes"#;
        assert_eq!(node_list(1)(input), Ok(("", vec![
            Node { name: "some".to_string(), children: vec![
                Node { name: "list".to_string(), children: vec![]}]},
            Node { name: "of".to_string(),       children: vec![
                Node { name: "indented".to_string(), children: vec![]}]},
            Node { name: "branch".to_string(),    children: vec![
                Node { name: "nodes".to_string(),    children: vec![]}]},
        ])))
    }

    #[test]
    fn test_key() {
        let input = "systemctl";
        assert_eq!(key(input), Ok(("", input)));
    }

    #[test]
    fn test_key_with_leading_whitespace() {
        let input = "  systemctl";
        assert_eq!(
            key(input),
            Err(nom::Err::Error(nom::error::Error { input, code: nom::error::ErrorKind::IsNot }))
        );
    }

    // Deliberately accept trailing whitespace- this is a decision that could change later
    #[test]
    fn test_key_with_trailing_whitespace() {
        let input = "systemctl  ";
        assert_eq!(key(input), Ok(("", "systemctl  ")));
    }

    #[test]
    fn test_key_with_trailing_newline() {
        let input = "systemctl\n";
        assert_eq!(key(input), Ok(("\n", "systemctl")));
    }

    #[test]
    fn test_branch() {
        let input = "  systemctl\n    user";
        assert_eq!(node(1)(input), Ok(("", Node {
            name: "systemctl".to_string(),
            children: vec![Node { name: "user".to_string(), children: vec![]}],
        })));
    }

    #[test]
    fn test_leaf() {
        let input = "  systemctl";
        assert_eq!(node(1)(input), Ok(("", Node {
            name: "systemctl".to_string(),
            children: vec![],
        })));
    }

    #[test]
    fn test_strip_empty_lines() {
        let expected_result = "interior";
        let input = format!(" \n  \n\n{}\n \n  \n\n", expected_result);
        assert_eq!(strip_empty_lines(key)(&input), Ok(("", expected_result)));
    }

    #[test]
    fn test_single_key_node_unindented() {
        let input = "systemctl";
        assert_eq!(node(0)(input), Ok(("", Node { name: "systemctl".to_string(), children: vec![] })));
    }

    #[test]
    fn test_single_key_node_indented() {
        let input = "  systemctl";
        assert_eq!(node(1)(input), Ok(("", Node { name: "systemctl".to_string(), children: vec![] })));
    }

    #[test]
    fn test_node() {
        let input =
r#"systemctl
  --user
    restart
      pulseaudio
  --system
    restart
      iwd"#;
        assert_eq!(node(0)(input), Ok(("", Node {
            name: "systemctl".to_string(),
            children: vec![
                Node {
                    name: "--user".to_string(),
                    children: vec![
                        Node {
                            name: "restart".to_string(),
                            children: vec![
                                Node {
                                    name: "pulseaudio".to_string(),
                                    children: vec![],
                                },
                            ],
                        },
                    ],
                },
                Node {
                    name: "--system".to_string(),
                    children: vec![
                        Node {
                            name: "restart".to_string(),
                            children: vec![
                                Node {
                                    name: "iwd".to_string(),
                                    children: vec![],
                                },
                            ],
                        },
                    ],
                },
            ],
        })));
    }

}
