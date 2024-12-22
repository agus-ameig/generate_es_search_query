
#[cfg(test)]
mod tests {
    use es_search_query_procedural_macros::create_search_query;
    use serde_json::Value;

    #[create_search_query(
        r#"{
            "query": {
            "match": {
                "title":"{{search}}"
            }
        }}"#
    )]
    struct TestSearchParams {
        search: String,
    }

    #[create_search_query(
        r#"{
            "query": {
            "match": {
                "title": "{{search}}",
                "order_n": {{order_n}}
            }
        }}"#
   )]
    struct MoreComplicatedTestSearchParams {
        search: String,
        order_n: u16
    }


    #[test]
    fn basic_query_substitution() {
        let t = TestSearchParams {
            search: "a search query".to_string(),
        };
        assert_eq!(t.get_search_query(), r#"{"query":{"match":{"title":"a search query"}}}"#);
    }

    #[test]
    fn parameters_should_be_escaped() {
        let t = TestSearchParams {
            search: r#"a search query", "another_property": "another"#.to_string()
        };

        let a = serde_json::from_str::<Value>(&t.get_search_query()).unwrap();

        assert_eq!(a["query"]["match"]["title"].to_string(), "\"a search query\\\", \\\"another_property\\\": \\\"another\"".to_string())
    }

    #[test]
    fn numbers_in_query_substitution() {
        let t = MoreComplicatedTestSearchParams {
            search: "a".to_string(),
            order_n: 16
        };
        assert_eq!(t.get_search_query(), r#"{"query":{"match":{"title":"a","order_n":16}}}"#.to_string())
    }
}
