use crate::features::board::Board;
use crate::services::storage;
use futures::{FutureExt, future::Shared};
use gpui_kit::{App, BorrowAppContext, Global, Task};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum Route {
    #[default]
    Board,
    Chat,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct Workspace {
    pub version: u32,
    pub route: Route,
    pub board: Board,
}

impl Default for Workspace {
    fn default() -> Self {
        Self {
            version: 1,
            route: Route::Board,
            board: Board::default(),
        }
    }
}

pub struct AppState {
    pub workspace: Workspace,
    pub warning: Option<String>,
    pub save_error: Option<String>,
    path: PathBuf,
    revision: u64,
    save_task: Option<Shared<Task<Result<(), String>>>>,
}

impl Global for AppState {}

pub fn initialize(loaded: storage::LoadedWorkspace, path: PathBuf, cx: &mut App) {
    cx.set_global(AppState {
        workspace: loaded.workspace,
        warning: loaded.warning,
        save_error: None,
        path,
        revision: 0,
        save_task: None,
    });
    cx.on_app_quit(|cx| {
        let pending = cx.global_mut::<AppState>().save_task.take();
        async move {
            if let Some(task) = pending
                && let Err(error) = task.await
            {
                eprintln!("Unable to save workspace before quitting: {error}");
            }
        }
    })
    .detach();
}

pub fn update(cx: &mut App, change: impl FnOnce(&mut Workspace)) {
    let (snapshot, path, revision, previous) = cx.update_global::<AppState, _>(|state, _| {
        change(&mut state.workspace);
        state.revision += 1;
        (
            state.workspace.clone(),
            state.path.clone(),
            state.revision,
            state.save_task.take(),
        )
    });
    // Keep the whole chain off the foreground executor, including at shutdown.
    let task = cx
        .background_executor()
        .spawn(async move {
            if let Some(previous) = previous {
                let _ = previous.await;
            }
            storage::save(&path, &snapshot).map_err(|error| error.to_string())
        })
        .shared();
    let observer = task.clone();
    cx.spawn(async move |cx| {
        let result = observer.await;
        cx.update(|cx| {
            cx.update_global::<AppState, _>(|state, _| {
                if state.revision == revision {
                    state.save_error = result
                        .err()
                        .map(|error| format!("Changes are only in memory; saving failed: {error}"));
                }
            });
        });
    })
    .detach();
    cx.global_mut::<AppState>().save_task = Some(task);
}
