pub mod repo;
pub mod skills;

/// Default provider id for Mesh templates when no explicit provider is given.
pub const MESH_DEFAULT_PROVIDER: &str = "claude-code";

#[cfg(test)]
mod tests {
    use super::repo::*;
    use crate::services::database::DatabaseService;
    use crate::test_utils::TestCase;

    fn db() -> DatabaseService {
        DatabaseService::open_in_memory().unwrap()
    }

    fn seed_session(db: &DatabaseService) -> i64 {
        db.create_session(
            None,
            Some("mesh::test"),
            "/tmp",
            "ignore",
            None,
            None,
            None,
            None,
        )
        .unwrap()
    }

    #[test]
    fn should_create_and_list_floors() {
        let mut t = TestCase::new("should_create_and_list_floors");
        t.phase("Act");
        let db = db();
        let f = create_floor(&db, "Main").unwrap();
        t.phase("Assert");
        t.ok("id is positive", f.id > 0);
        t.eq("name matches", f.name.as_str(), "Main");
        let list = list_floors(&db).unwrap();
        t.len("one floor listed", &list, 1);
    }

    #[test]
    fn should_create_template_inside_floor() {
        let mut t = TestCase::new("should_create_template_inside_floor");
        t.phase("Seed");
        let db = db();
        let f = create_floor(&db, "F").unwrap();
        t.phase("Act");
        let tpl =
            create_template(&db, f.id, "Reader", "Read files", None, true, "claude-code").unwrap();
        t.phase("Assert");
        t.eq("name matches", tpl.name.as_str(), "Reader");
        t.eq("floor_id matches", tpl.floor_id, f.id);
        t.ok("use_worktree is true", tpl.use_worktree);
        t.eq("provider matches", tpl.provider.as_str(), "claude-code");
    }

    #[test]
    fn should_create_browser_template() {
        let mut t = TestCase::new("should_create_browser_template");
        t.phase("Seed");
        let db = db();
        let f = create_floor(&db, "F").unwrap();
        t.phase("Act");
        let tpl = create_template(
            &db,
            f.id,
            "Docs",
            "https://anthropic.com",
            None,
            false,
            "browser",
        )
        .unwrap();
        t.phase("Assert");
        t.eq("provider is browser", tpl.provider.as_str(), "browser");
    }

    #[test]
    fn should_create_graph_with_nodes_and_edges() {
        let mut t = TestCase::new("should_create_graph_with_nodes_and_edges");
        t.phase("Seed");
        let db = db();
        let f = create_floor(&db, "F").unwrap();
        let tpl = create_template(&db, f.id, "T", "p", None, true, "claude-code").unwrap();
        let g = create_graph(&db, f.id, "G", "claude-code").unwrap();
        t.phase("Act");
        let n1 = add_node(&db, g.id, tpl.id, "A", 0.0, 0.0).unwrap();
        let n2 = add_node(&db, g.id, tpl.id, "B", 100.0, 0.0).unwrap();
        let e = add_edge(&db, g.id, n1.id, n2.id, None, None).unwrap();
        t.phase("Assert");
        t.eq("edge from is n1", e.from_node_id, n1.id);
        t.eq("edge to is n2", e.to_node_id, n2.id);
        t.len("two nodes listed", &list_nodes(&db, g.id).unwrap(), 2);
        t.len("one edge listed", &list_edges(&db, g.id).unwrap(), 1);
    }

    #[test]
    fn should_reject_duplicate_node_name_in_same_graph() {
        let mut t = TestCase::new("should_reject_duplicate_node_name_in_same_graph");
        t.phase("Seed");
        let db = db();
        let f = create_floor(&db, "F").unwrap();
        let tpl = create_template(&db, f.id, "T", "p", None, true, "claude-code").unwrap();
        let g = create_graph(&db, f.id, "G", "claude-code").unwrap();
        add_node(&db, g.id, tpl.id, "A", 0.0, 0.0).unwrap();
        t.phase("Act");
        let dup = add_node(&db, g.id, tpl.id, "A", 50.0, 50.0);
        t.phase("Assert");
        t.is_err("duplicate display_name violates UNIQUE", &dup);
    }

    #[test]
    fn should_cascade_delete_graph_removes_nodes_and_edges() {
        let mut t = TestCase::new("should_cascade_delete_graph_removes_nodes_and_edges");
        t.phase("Seed");
        let db = db();
        let f = create_floor(&db, "F").unwrap();
        let tpl = create_template(&db, f.id, "T", "p", None, true, "claude-code").unwrap();
        let g = create_graph(&db, f.id, "G", "claude-code").unwrap();
        let n1 = add_node(&db, g.id, tpl.id, "A", 0.0, 0.0).unwrap();
        let n2 = add_node(&db, g.id, tpl.id, "B", 0.0, 0.0).unwrap();
        add_edge(&db, g.id, n1.id, n2.id, None, None).unwrap();
        t.phase("Act");
        delete_graph(&db, g.id).unwrap();
        t.phase("Assert");
        t.empty("nodes cleaned up", &list_nodes(&db, g.id).unwrap());
        t.empty("edges cleaned up", &list_edges(&db, g.id).unwrap());
    }

    #[test]
    fn should_persist_width_and_height_on_resize() {
        let mut t = TestCase::new("should_persist_width_and_height_on_resize");
        t.phase("Seed");
        let db = db();
        let f = create_floor(&db, "F").unwrap();
        let tpl = create_template(&db, f.id, "T", "p", None, true, "claude-code").unwrap();
        let g = create_graph(&db, f.id, "G", "claude-code").unwrap();
        let n = add_node(&db, g.id, tpl.id, "A", 0.0, 0.0).unwrap();
        t.none("fresh node has no width", &n.width);
        t.none("fresh node has no height", &n.height);
        t.phase("Act");
        resize_node(&db, n.id, 480.0, 320.0).unwrap();
        t.phase("Assert");
        let nodes = list_nodes(&db, g.id).unwrap();
        t.len("one node persists", &nodes, 1);
        t.eq("width persisted", nodes[0].width, Some(480.0));
        t.eq("height persisted", nodes[0].height, Some(320.0));
    }

    #[test]
    fn should_create_and_get_run() {
        let mut t = TestCase::new("should_create_and_get_run");
        t.phase("Seed");
        let db = db();
        let f = create_floor(&db, "F").unwrap();
        let tpl = create_template(&db, f.id, "T", "p", None, true, "claude-code").unwrap();
        let g = create_graph(&db, f.id, "G", "claude-code").unwrap();
        let n = add_node(&db, g.id, tpl.id, "A", 0.0, 0.0).unwrap();
        t.phase("Act");
        let run = create_run(&db, g.id, n.id, Some("do work")).unwrap();
        t.phase("Assert");
        t.ok("run id is positive", run.id > 0);
        t.eq("graph_id matches", run.graph_id, g.id);
        t.eq("entry_node_id matches", run.entry_node_id, n.id);
        t.eq(
            "initial_prompt matches",
            run.initial_prompt.as_deref(),
            Some("do work"),
        );
        t.eq("status is running", run.status.as_str(), "running");
        t.some("started_at set", &run.started_at);
        let fetched = get_run(&db, run.id).unwrap();
        t.eq("get_run round-trip id", fetched.id, run.id);
        t.eq(
            "get_run round-trip status",
            fetched.status.as_str(),
            "running",
        );
    }

    #[test]
    fn should_finish_run_with_status() {
        let mut t = TestCase::new("should_finish_run_with_status");
        t.phase("Seed");
        let db = db();
        let f = create_floor(&db, "F").unwrap();
        let tpl = create_template(&db, f.id, "T", "p", None, true, "claude-code").unwrap();
        let g = create_graph(&db, f.id, "G", "claude-code").unwrap();
        let n = add_node(&db, g.id, tpl.id, "A", 0.0, 0.0).unwrap();
        let run = create_run(&db, g.id, n.id, None).unwrap();
        t.phase("Act");
        finish_run(&db, run.id, "completed").unwrap();
        t.phase("Assert");
        let fetched = get_run(&db, run.id).unwrap();
        t.eq("status updated", fetched.status.as_str(), "completed");
        t.some("finished_at set", &fetched.finished_at);
    }

    #[test]
    fn should_record_and_list_run_sessions() {
        let mut t = TestCase::new("should_record_and_list_run_sessions");
        t.phase("Seed");
        let db = db();
        let f = create_floor(&db, "F").unwrap();
        let tpl = create_template(&db, f.id, "T", "p", None, true, "claude-code").unwrap();
        let g = create_graph(&db, f.id, "G", "claude-code").unwrap();
        let n1 = add_node(&db, g.id, tpl.id, "A", 0.0, 0.0).unwrap();
        let n2 = add_node(&db, g.id, tpl.id, "B", 0.0, 0.0).unwrap();
        let run = create_run(&db, g.id, n1.id, None).unwrap();
        let s1 = seed_session(&db);
        let s2 = seed_session(&db);
        t.phase("Act");
        let rs1 = record_run_session(&db, run.id, n1.id, s1).unwrap();
        let rs2 = record_run_session(&db, run.id, n2.id, s2).unwrap();
        t.phase("Assert");
        t.ok("rs1 id is positive", rs1.id > 0);
        t.ok("rs ids differ", rs1.id != rs2.id);
        let list = list_run_sessions(&db, run.id).unwrap();
        t.len("two run sessions listed", &list, 2);
        t.eq("first session_id matches", list[0].session_id, s1);
        t.eq("second session_id matches", list[1].session_id, s2);
    }

    #[test]
    fn should_reject_duplicate_run_session_for_same_node() {
        let mut t = TestCase::new("should_reject_duplicate_run_session_for_same_node");
        t.phase("Seed");
        let db = db();
        let f = create_floor(&db, "F").unwrap();
        let tpl = create_template(&db, f.id, "T", "p", None, true, "claude-code").unwrap();
        let g = create_graph(&db, f.id, "G", "claude-code").unwrap();
        let n = add_node(&db, g.id, tpl.id, "A", 0.0, 0.0).unwrap();
        let run = create_run(&db, g.id, n.id, None).unwrap();
        let s1 = seed_session(&db);
        let s2 = seed_session(&db);
        record_run_session(&db, run.id, n.id, s1).unwrap();
        t.phase("Act");
        let dup = record_run_session(&db, run.id, n.id, s2);
        t.phase("Assert");
        t.is_err("UNIQUE(run_id, node_id) rejects second binding", &dup);
    }

    #[test]
    fn should_find_only_active_run() {
        let mut t = TestCase::new("should_find_only_active_run");
        t.phase("Seed");
        let db = db();
        let f = create_floor(&db, "F").unwrap();
        let tpl = create_template(&db, f.id, "T", "p", None, true, "claude-code").unwrap();
        let g = create_graph(&db, f.id, "G", "claude-code").unwrap();
        let n = add_node(&db, g.id, tpl.id, "A", 0.0, 0.0).unwrap();
        let r1 = create_run(&db, g.id, n.id, None).unwrap();
        finish_run(&db, r1.id, "completed").unwrap();
        let r2 = create_run(&db, g.id, n.id, None).unwrap();
        t.phase("Act");
        let active = find_active_run(&db, g.id).unwrap();
        t.phase("Assert");
        t.eq("returns the running run", active.map(|r| r.id), Some(r2.id));
    }

    #[test]
    fn should_return_none_when_no_active_run() {
        let mut t = TestCase::new("should_return_none_when_no_active_run");
        t.phase("Seed");
        let db = db();
        let f = create_floor(&db, "F").unwrap();
        let g = create_graph(&db, f.id, "G", "claude-code").unwrap();
        t.phase("Act");
        let active = find_active_run(&db, g.id).unwrap();
        t.phase("Assert");
        t.none("no active run", &active);
    }

    #[test]
    fn should_update_graph_provider() {
        let mut t = TestCase::new("should_update_graph_provider");
        t.phase("Seed");
        let db = db();
        let f = create_floor(&db, "F").unwrap();
        let g = create_graph(&db, f.id, "G", "claude-code").unwrap();
        t.eq("starts on default", g.provider.as_str(), "claude-code");
        t.phase("Act");
        update_graph_provider(&db, g.id, "codex").unwrap();
        t.phase("Assert");
        let listed = list_graphs(&db, f.id).unwrap();
        t.eq(
            "provider was switched",
            listed[0].provider.as_str(),
            "codex",
        );
    }

    #[test]
    fn should_create_note_and_list() {
        let mut t = TestCase::new("should_create_note_and_list");
        t.phase("Seed");
        let db = db();
        let f = create_floor(&db, "F").unwrap();
        let g = create_graph(&db, f.id, "G", "claude-code").unwrap();
        t.phase("Act");
        let n = add_note(&db, g.id, "Notes", 10.0, 20.0).unwrap();
        t.phase("Assert");
        t.eq("graph matches", n.graph_id, g.id);
        t.eq("name matches", n.name.as_str(), "Notes");
        t.eq("content empty", n.content.as_str(), "");
        t.len("listed", &list_notes(&db, g.id).unwrap(), 1);
    }

    #[test]
    fn should_set_and_round_trip_note_content() {
        let mut t = TestCase::new("should_set_and_round_trip_note_content");
        t.phase("Seed");
        let db = db();
        let f = create_floor(&db, "F").unwrap();
        let g = create_graph(&db, f.id, "G", "claude-code").unwrap();
        let n = add_note(&db, g.id, "Spec", 0.0, 0.0).unwrap();
        t.phase("Act");
        set_note_content(&db, n.node_id, "# title\nbody").unwrap();
        t.phase("Assert");
        let fetched = get_note(&db, n.node_id).unwrap();
        t.eq("content updated", fetched.content.as_str(), "# title\nbody");
    }

    #[test]
    fn should_rename_note_and_keep_display_name_in_sync() {
        let mut t = TestCase::new("should_rename_note_and_keep_display_name_in_sync");
        t.phase("Seed");
        let db = db();
        let f = create_floor(&db, "F").unwrap();
        let g = create_graph(&db, f.id, "G", "claude-code").unwrap();
        let n = add_note(&db, g.id, "Old", 0.0, 0.0).unwrap();
        t.phase("Act");
        rename_note(&db, n.node_id, "New").unwrap();
        t.phase("Assert");
        t.eq(
            "note name",
            get_note(&db, n.node_id).unwrap().name.as_str(),
            "New",
        );
        let nodes = list_nodes(&db, g.id).unwrap();
        t.eq(
            "display_name mirrored",
            nodes[0].display_name.as_str(),
            "New",
        );
    }

    #[test]
    fn should_dedupe_note_display_name_within_graph() {
        let mut t = TestCase::new("should_dedupe_note_display_name_within_graph");
        t.phase("Seed");
        let db = db();
        let f = create_floor(&db, "F").unwrap();
        let g = create_graph(&db, f.id, "G", "claude-code").unwrap();
        add_note(&db, g.id, "Note", 0.0, 0.0).unwrap();
        t.phase("Act");
        let dup = add_note(&db, g.id, "Note", 50.0, 50.0).unwrap();
        t.phase("Assert");
        t.eq(
            "second note got numeric suffix",
            dup.name.as_str(),
            "Note 2",
        );
    }

    #[test]
    fn should_cascade_delete_note_when_node_removed() {
        let mut t = TestCase::new("should_cascade_delete_note_when_node_removed");
        t.phase("Seed");
        let db = db();
        let f = create_floor(&db, "F").unwrap();
        let g = create_graph(&db, f.id, "G", "claude-code").unwrap();
        let n = add_note(&db, g.id, "Doomed", 0.0, 0.0).unwrap();
        t.phase("Act");
        remove_node(&db, n.node_id).unwrap();
        t.phase("Assert");
        t.empty("note row gone", &list_notes(&db, g.id).unwrap());
    }

    #[test]
    fn should_reuse_floor_note_template_on_repeat_adds() {
        let mut t = TestCase::new("should_reuse_floor_note_template_on_repeat_adds");
        t.phase("Seed");
        let db = db();
        let f = create_floor(&db, "F").unwrap();
        let g = create_graph(&db, f.id, "G", "claude-code").unwrap();
        let a = add_note(&db, g.id, "A", 0.0, 0.0).unwrap();
        let b = add_note(&db, g.id, "B", 50.0, 0.0).unwrap();
        t.phase("Assert");
        let nodes = list_nodes(&db, g.id).unwrap();
        let ta = nodes
            .iter()
            .find(|n| n.id == a.node_id)
            .unwrap()
            .template_id;
        let tb = nodes
            .iter()
            .find(|n| n.id == b.node_id)
            .unwrap()
            .template_id;
        t.eq("template_id is shared", ta, tb);
    }

    #[test]
    fn should_save_and_list_annotations() {
        let mut t = TestCase::new("should_save_and_list_annotations");
        t.phase("Seed");
        let db = db();
        let f = create_floor(&db, "F").unwrap();
        let g = create_graph(&db, f.id, "G", "claude-code").unwrap();
        t.phase("Act");
        save_annotations(
            &db,
            g.id,
            &[
                NewAnnotation {
                    kind: "path".into(),
                    payload: "{}".into(),
                    z_index: 0,
                },
                NewAnnotation {
                    kind: "sticky".into(),
                    payload: "{\"text\":\"hi\"}".into(),
                    z_index: 1,
                },
            ],
        )
        .unwrap();
        t.phase("Assert");
        let list = list_annotations(&db, g.id).unwrap();
        t.len("two annotations persisted", &list, 2);
    }
}
