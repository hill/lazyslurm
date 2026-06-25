//! Render smoke tests. Each tab is drawn into an in-memory TestBackend so a
//! layout or unicode-width panic shows up here rather than only in a live
//! terminal. They also pin that the right content reaches the screen.

use lazyslurm::models::{AcctDetail, AcctEntry, Node, Partition};
use lazyslurm::ui::{ActiveTab, App, AppState, render_app, tab_rects};
use ratatui::{Terminal, backend::TestBackend, layout::Rect};

/// Concatenate the buffer's cell contents row by row so substring checks work.
fn rendered_text(app: &App) -> String {
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| render_app(frame, app)).unwrap();

    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

fn sample_node() -> Node {
    Node {
        name: "gpu-node-01".into(),
        state: "mixed".into(),
        cpus_alloc: 12,
        cpus_idle: 20,
        cpus_other: 0,
        cpus_total: 32,
        memory_mb: Some(257_000),
        free_mem_mb: Some(120_000),
        gres: Some("gpu:a100:4".into()),
        partition: "gpu".into(),
    }
}

fn sample_partition() -> Partition {
    Partition {
        name: "batch".into(),
        is_default: true,
        availability: "up".into(),
        nodes_alloc: 10,
        nodes_idle: 20,
        nodes_other: 2,
        nodes_total: 32,
        time_limit: "7-00:00:00".into(),
    }
}

fn sample_entry() -> AcctEntry {
    AcctEntry {
        job_id: "1001".into(),
        name: "train-resnet".into(),
        state: "COMPLETED".into(),
        exit_code: "0:0".into(),
        elapsed: "00:30:12".into(),
        start: "2026-06-25T09:00:00".into(),
        end: "2026-06-25T09:30:12".into(),
    }
}

#[test]
fn tab_bar_shows_every_tab() {
    let app = App::new();
    let text = rendered_text(&app);
    for label in ["Jobs", "Nodes", "Partitions", "History"] {
        assert!(text.contains(label), "tab bar missing {label}");
    }
}

#[test]
fn status_bar_has_no_middot_separator() {
    let app = App::new();
    assert!(
        !rendered_text(&app).contains('·'),
        "the · separator should be gone"
    );
}

#[test]
fn clicking_a_tab_label_hits_that_tab() {
    // Tie the click hit-test to where the label actually renders: find the
    // column of "Nodes" in the status row, then assert tab_rects maps it back.
    let app = App::new();
    let area = Rect::new(0, 0, 120, 40);
    let backend = TestBackend::new(area.width, area.height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| render_app(frame, &app)).unwrap();

    let buf = terminal.backend().buffer();
    let width = buf.area.width as usize;
    let row0: String = buf.content()[..width].iter().map(|c| c.symbol()).collect();

    let col = row0
        .find("Nodes")
        .expect("Nodes tab should render in the status row") as u16;
    assert_eq!(tab_rects(area).hit(col, 0), Some(ActiveTab::Nodes));
    // A click far to the left, over the title, hits no tab.
    assert_eq!(tab_rects(area).hit(2, 0), None);
}

#[test]
fn nodes_tab_renders_node_row() {
    let mut app = App::new();
    app.active_tab = ActiveTab::Nodes;
    app.nodes = vec![sample_node()];
    let text = rendered_text(&app);
    assert!(text.contains("gpu-node-01"));
    assert!(text.contains("mixed"));
    assert!(text.contains("gpu:a100:4"));
}

#[test]
fn partitions_tab_marks_default_and_shows_limit() {
    let mut app = App::new();
    app.active_tab = ActiveTab::Partitions;
    app.partitions = vec![sample_partition()];
    let text = rendered_text(&app);
    assert!(
        text.contains("batch*"),
        "default partition should carry the star"
    );
    assert!(text.contains("7-00:00:00"));
}

#[test]
fn history_tab_renders_entry() {
    let mut app = App::new();
    app.active_tab = ActiveTab::History;
    app.history = vec![sample_entry()];
    let text = rendered_text(&app);
    assert!(text.contains("train-resnet"));
    assert!(text.contains("00:30:12"));
}

#[test]
fn history_tab_explains_missing_accounting() {
    let mut app = App::new();
    app.active_tab = ActiveTab::History;
    app.history_error = Some("sacct failed: accounting disabled".into());
    let text = rendered_text(&app);
    assert!(text.contains("Accounting not available"));
}

fn sample_detail() -> AcctDetail {
    AcctDetail {
        job_id: "1001".into(),
        name: "train-resnet".into(),
        user: "ada".into(),
        account: "ml-lab".into(),
        partition: "gpu".into(),
        node_list: "gpu-node-01".into(),
        alloc_cpus: "8".into(),
        req_mem: "32Gn".into(),
        max_rss: Some("18.4G".into()),
        total_cpu: "02:10:00".into(),
        state: "COMPLETED".into(),
        exit_code: "0:0".into(),
        submit: "2026-06-25T08:59:00".into(),
        start: "2026-06-25T09:00:00".into(),
        end: "2026-06-25T09:30:12".into(),
        elapsed: "00:30:12".into(),
        work_dir: "/home/ada/runs".into(),
    }
}

#[test]
fn history_detail_renders_rich_fields() {
    let mut app = App::new();
    app.active_tab = ActiveTab::History;
    app.state = AppState::HistoryDetail;
    app.history_detail_id = Some("1001".into());
    app.history_detail = Some(sample_detail());
    let text = rendered_text(&app);
    assert!(text.contains("ml-lab"), "account shown");
    assert!(text.contains("gpu-node-01"), "node list shown");
    assert!(text.contains("18.4G"), "MaxRSS (used memory) shown");
    assert!(text.contains("/home/ada/runs"), "work dir shown");
    // Best-effort log for a path that doesn't exist must show the honest miss.
    assert!(text.contains("No log file found"));
}

#[test]
fn history_detail_shows_loading_before_fetch_lands() {
    let mut app = App::new();
    app.state = AppState::HistoryDetail;
    app.history_detail_id = Some("1001".into());
    app.history_detail_loading = true;
    let text = rendered_text(&app);
    assert!(text.contains("Loading job detail"));
}

#[test]
fn jobs_filter_line_and_count_render() {
    use lazyslurm::models::{Job, JobState};
    let mut app = App::new();
    app.job_list.update(vec![
        Job::new(
            "100".into(),
            "train_resnet".into(),
            "u".into(),
            JobState::Running,
        ),
        Job::new(
            "101".into(),
            "eval_run".into(),
            "u".into(),
            JobState::Running,
        ),
    ]);
    app.filter_query = "train".into();
    let text = rendered_text(&app);
    assert!(text.contains("train"), "filter query shown");
    assert!(text.contains("1/2"), "filtered/total count shown");
}

#[test]
fn pinned_job_shows_star_marker() {
    use lazyslurm::models::{Job, JobState};
    let mut app = App::new();
    app.job_list.update(vec![Job::new(
        "100".into(),
        "train".into(),
        "u".into(),
        JobState::Running,
    )]);
    app.pinned.insert("100".into());
    assert!(rendered_text(&app).contains('★'), "pinned row shows a star");
}

#[test]
fn raw_log_view_shows_plain_log() {
    use std::io::Write;

    let path = std::env::temp_dir().join("lazyslurm_rawtest.log");
    let mut f = std::fs::File::create(&path).unwrap();
    writeln!(f, "first log line").unwrap();
    writeln!(f, "second log line").unwrap();
    f.flush().unwrap();

    let mut app = App::new();
    app.state = AppState::RawLog;
    app.raw_log_paths = vec![path.to_string_lossy().into_owned()];
    app.log_follow = true;

    let text = rendered_text(&app);
    assert!(text.contains("RAW"), "raw badge shown");
    assert!(text.contains("second log line"), "log content shown");

    std::fs::remove_file(&path).ok();
}

#[test]
fn cluster_tabs_render_when_empty() {
    // An empty list with no error must not panic and should show the hint.
    for tab in [ActiveTab::Nodes, ActiveTab::Partitions] {
        let mut app = App::new();
        app.active_tab = tab;
        let _ = rendered_text(&app);
    }
}
