#[cfg(test)]
mod tests {
    use rgt::graph::GraphEngine;

    #[test]
    fn test_reverse_edge_downstream_dependents() {
        let mut engine = GraphEngine::new();
        engine.get_or_add_node("root1");
        engine.get_or_add_node("derived1");
        engine.get_or_add_node("derived2");

        let _ = engine.add_derivation_edge("root1", "derived1", "ADD");
        let _ = engine.add_derivation_edge("derived1", "derived2", "MULTIPLY");

        let dependents = engine.get_downstream_dependents("root1");
        assert_eq!(dependents.len(), 2);
        assert!(dependents.contains(&"derived1".to_string()));
        assert!(dependents.contains(&"derived2".to_string()));
    }

    #[test]
    fn test_cycle_prevention() {
        let mut engine = GraphEngine::new();
        engine.get_or_add_node("nodeA");
        engine.get_or_add_node("nodeB");

        assert!(engine.add_derivation_edge("nodeA", "nodeB", "OP1").is_ok());
        assert!(engine.add_derivation_edge("nodeB", "nodeA", "OP2").is_err());
    }
}
