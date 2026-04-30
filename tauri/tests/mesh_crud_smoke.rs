//! End-to-end CRUD smoke test for the Mesh infrastructure (M1).
//! Verifies that all new entities persist correctly and cascades clean up children.

use orbit_lib::models::Floor;
use orbit_lib::services::database::DatabaseService;
use orbit_lib::services::mesh::repo::{
    add_edge, add_node, create_floor, create_graph, create_template, delete_graph,
    list_annotations, list_edges, list_nodes, resize_node, save_annotations, NewAnnotation,
};

#[test]
fn full_mesh_lifecycle_persists_and_cleans_up() {
    let db = DatabaseService::open_in_memory().unwrap();

    // 1. Create floor
    let f: Floor = create_floor(&db, "Design Floor").unwrap();
    assert!(f.id > 0);

    // 2. Create 3 templates
    let t_reader =
        create_template(&db, f.id, "Reader", "Read code", None, true, "claude-code").unwrap();
    let t_planner =
        create_template(&db, f.id, "Planner", "Plan work", None, true, "claude-code").unwrap();
    let t_impl =
        create_template(&db, f.id, "Impl", "Implement", None, true, "claude-code").unwrap();

    // 3. Create graph with 3 nodes
    let g = create_graph(&db, f.id, "Pipeline", "claude-code").unwrap();
    let n_read = add_node(&db, g.id, t_reader.id, "A", 0.0, 0.0).unwrap();
    let n_plan = add_node(&db, g.id, t_planner.id, "B", 200.0, 0.0).unwrap();
    let n_impl = add_node(&db, g.id, t_impl.id, "C", 400.0, 0.0).unwrap();

    // 4. Connect A→B→C
    add_edge(&db, g.id, n_read.id, n_plan.id, None, None).unwrap();
    add_edge(&db, g.id, n_plan.id, n_impl.id, None, None).unwrap();

    // 5. Add annotations (drawing layer data)
    save_annotations(
        &db,
        g.id,
        &[
            NewAnnotation {
                kind: "path".into(),
                payload: "{\"points\":[[0,0],[10,10]]}".into(),
                z_index: 0,
            },
            NewAnnotation {
                kind: "sticky".into(),
                payload: "{\"text\":\"remember to X\"}".into(),
                z_index: 1,
            },
        ],
    )
    .unwrap();

    // 6. Verify reads
    assert_eq!(list_nodes(&db, g.id).unwrap().len(), 3);
    assert_eq!(list_edges(&db, g.id).unwrap().len(), 2);
    assert_eq!(list_annotations(&db, g.id).unwrap().len(), 2);

    // 7. Resize a node and verify persistence round-trip
    resize_node(&db, n_read.id, 480.0, 320.0).unwrap();
    let nodes = list_nodes(&db, g.id).unwrap();
    let resized = nodes.iter().find(|n| n.id == n_read.id).unwrap();
    assert_eq!(resized.width, Some(480.0));
    assert_eq!(resized.height, Some(320.0));

    // 8. Delete graph cascades
    delete_graph(&db, g.id).unwrap();
    assert_eq!(list_nodes(&db, g.id).unwrap().len(), 0);
    assert_eq!(list_edges(&db, g.id).unwrap().len(), 0);
    assert_eq!(list_annotations(&db, g.id).unwrap().len(), 0);
}
