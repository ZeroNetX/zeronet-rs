use serde_json::{json, Value};

pub fn calc_diff(old: &str, new: &str) -> Vec<Value> {
    let res = diff::chars(old, new);
    let mut diff = Vec::new();
    let mut prev = "";
    let mut count = 0i32;
    let mut added_str = String::new();
    for diff_res in res {
        match diff_res {
            diff::Result::Left(_) => {
                if prev != "-" {
                    if prev == "+" {
                        diff.push(Value::Array(vec![json!(prev), json!(added_str)]));
                        added_str = String::new();
                    } else if count > 0 {
                        diff.push(Value::Array(vec![json!(prev), json!(count)]));
                    }
                    count = 0;
                }
                prev = "-";
                count += 1;
            }
            diff::Result::Both(_, _) => {
                if prev != "=" {
                    if prev == "+" {
                        diff.push(Value::Array(vec![json!(prev), json!(added_str)]));
                        added_str = String::new();
                    } else if count > 0 {
                        diff.push(Value::Array(vec![json!(prev), json!(count)]));
                    }
                    count = 0;
                }
                prev = "=";
                count += 1;
            }
            diff::Result::Right(c) => {
                if prev != "+" && count > 0 {
                    diff.push(Value::Array(vec![json!(prev), json!(count)]));
                    count = 0;
                }
                prev = "+";
                count += 1;
                added_str.push(c);
            }
        };
    }
    if prev == "+" {
        diff.push(Value::Array(vec![json!(prev), json!(added_str)]));
    } else {
        diff.push(Value::Array(vec![json!(prev), json!(count)]));
    }
    diff
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calc_diff() {
        assert_eq!(
            calc_diff("", "one, two, three"),
            vec![Value::Array(vec![json!("+"), json!["one, two, three"]])],
        );
        assert_eq!(
            calc_diff("one, two, three", "one, two, three, four, five"),
            vec![
                Value::Array(vec![json!("="), json![15]]),
                Value::Array(vec![json!("+"), json![", four, five"]]),
            ]
        );
        assert_eq!(
            calc_diff("one, two, three, six", "one, two, three, four, five, six"),
            vec![
                Value::Array(vec![json!("="), json![17]]),
                Value::Array(vec![json!("+"), json!["four, five, "]]),
                Value::Array(vec![json!("="), json![3]]),
            ]
        );
        assert_eq!(
            calc_diff(
                "one, two, three, hmm, six",
                "one, two, three, four, five, six"
            ),
            vec![
                Value::Array(vec![json!("="), json![17]]),
                Value::Array(vec![json!("-"), json![3]]),
                Value::Array(vec![json!("+"), json!["four, five"]]),
                Value::Array(vec![json!("="), json![5]]),
            ]
        );
        assert_eq!(
            calc_diff("one, two, three", ""),
            vec![Value::Array(vec![json!("-"), json![15]])]
        );
    }
}
