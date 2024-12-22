#[cfg(test)]
mod test {
    use es_search_query_procedural_macros::generate_search_query;
    use generate_es_search_query::SearchQuery;

    #[generate_search_query(
        filter(terms { name = tags, data_type = String }),
        filter(multi_match {query = search, data_type = String, fields=searchable("field_a","field_b")})
    )]
    struct Test {
        search: String,
        tags: Vec<String>
    }

    #[test]
    fn test() {
        let t = Test {
            search: "b".to_string(),
            tags: vec!["a".to_string(),"b".to_string(),"c".to_string()]
        };
        let a =  t.get_search_query();
        println!("Q: {:?}", serde_json::to_string(&a));
    }
}
