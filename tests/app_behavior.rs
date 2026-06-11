use std::sync::Arc;

use lazyslurm::slurm::SlurmFixture;
use lazyslurm::ui::App;

fn fixture_app(name: &str) -> (App, Arc<SlurmFixture>) {
    let fixture = Arc::new(SlurmFixture::new(format!("tests/fixtures/{name}")));
    let app = App::with_executor(fixture.clone());
    (app, fixture)
}

#[tokio::test]
async fn cancel_applies_to_snapshotted_job_even_if_selection_moves() {
    let (mut app, fixture) = fixture_app("basic");
    app.refresh_jobs().await.unwrap();

    let target_id = app.selected_job.as_ref().unwrap().job_id.clone();
    app.open_cancel_popup();

    // The list keeps refreshing and the selection moves while the popup is open
    app.select_next_job();
    app.select_next_job();
    app.refresh_jobs().await.unwrap();

    app.confirm_cancel().await.unwrap();

    assert_eq!(*fixture.cancelled.lock().unwrap(), vec![target_id]);
}

#[tokio::test]
async fn dismissing_cancel_popup_cancels_nothing() {
    let (mut app, fixture) = fixture_app("basic");
    app.refresh_jobs().await.unwrap();

    app.open_cancel_popup();
    app.dismiss_cancel_popup();
    app.confirm_cancel().await.unwrap();

    assert!(fixture.cancelled.lock().unwrap().is_empty());
    assert!(app.cancel_target.is_none());
}

#[tokio::test]
async fn refresh_follows_selected_job_by_id() {
    let (mut app, _) = fixture_app("basic");
    app.refresh_jobs().await.unwrap();

    app.select_next_job();
    let selected_id = app.selected_job.as_ref().unwrap().job_id.clone();

    // A job above the selection disappears between refreshes
    app.job_list.jobs.remove(0);
    app.sync_selection(Some(&selected_id));

    assert_eq!(app.selected_job_index, 0);
    assert_eq!(app.selected_job.as_ref().unwrap().job_id, selected_id);
}

#[tokio::test]
async fn selection_clamps_when_job_list_shrinks() {
    let (mut app, _) = fixture_app("basic");
    app.refresh_jobs().await.unwrap();

    app.select_next_job();
    app.select_next_job();
    assert_eq!(app.selected_job_index, 2);

    // The whole list empties out
    let (empty_app, _) = fixture_app("empty");
    app.executor = empty_app.executor.clone();
    app.refresh_jobs().await.unwrap();

    assert_eq!(app.selected_job_index, 0);
    assert!(app.selected_job.is_none());
}
