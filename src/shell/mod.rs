use crate::{
    features::{
        board::Status,
        chat::{Chat, Role},
    },
    services::ai::ChatProvider,
    state::{self, AppState, Route},
};
use gpui_kit::{
    component::{
        button::*,
        input::{Input, InputState},
        *,
    },
    prelude::FluentBuilder,
    *,
};
use std::sync::Arc;

pub struct AppRoot {
    title: Entity<InputState>,
    assignee: Entity<InputState>,
    prompt: Entity<InputState>,
    chat: Chat,
    provider: Arc<dyn ChatProvider>,
    board_error: Option<String>,
    _state_subscription: Subscription,
}

impl AppRoot {
    pub fn new(
        provider: Arc<dyn ChatProvider>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            title: cx.new(|cx| InputState::new(window, cx).placeholder("Task title")),
            assignee: cx.new(|cx| InputState::new(window, cx).placeholder("Owner (optional)")),
            prompt: cx.new(|cx| InputState::new(window, cx).placeholder("Ask your assistant…")),
            chat: Chat::default(),
            provider,
            board_error: None,
            _state_subscription: cx.observe_global::<AppState>(|_, cx| cx.notify()),
        }
    }

    fn add_card(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let title = self.title.read(cx).value().to_string();
        let assignee = self.assignee.read(cx).value().to_string();
        let mut board = cx.global::<AppState>().workspace.board.clone();
        match board.add(&title, &assignee) {
            Ok(()) => {
                state::update(cx, |workspace| workspace.board = board);
                self.title
                    .update(cx, |input, cx| input.set_value("", window, cx));
                self.assignee
                    .update(cx, |input, cx| input.set_value("", window, cx));
                self.board_error = None;
            }
            Err(error) => self.board_error = Some(error.into()),
        }
        cx.notify();
    }

    fn send(&mut self, cx: &mut Context<Self>) {
        let prompt = self.prompt.read(cx).value().to_string();
        if !self.chat.begin(&prompt) {
            return;
        }
        let messages = self.chat.messages.clone();
        let provider = self.provider.clone();
        let request = cx
            .background_executor()
            .spawn(async move { provider.reply(&messages) });
        cx.spawn(async move |this, cx| {
            let result = request.await;
            let _ = this.update(cx, |this, cx| {
                this.chat.complete(result);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn board(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let board = cx.global::<AppState>().workspace.board.clone();
        v_flex()
            .gap_4()
            .child(div().text_xl().child("Team board"))
            .child("Local workspace · Notion sync and shared team accounts are not connected.")
            .child(
                h_flex()
                    .gap_2()
                    .child(div().flex_1().child(Input::new(&self.title)))
                    .child(div().w_48().child(Input::new(&self.assignee)))
                    .child(
                        Button::new("add-task")
                            .primary()
                            .label("Add task")
                            .on_click(cx.listener(|this, _, window, cx| this.add_card(window, cx))),
                    ),
            )
            .children(self.board_error.clone().map(|error| div().child(error)))
            .child(
                h_flex()
                    .items_start()
                    .gap_4()
                    .children(Status::ALL.map(|status| {
                        let cards: Vec<_> = board
                            .cards
                            .iter()
                            .filter(|card| card.status == status)
                            .collect();
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .gap_3()
                            .p_3()
                            .border_1()
                            .border_color(cx.theme().border)
                            .rounded_lg()
                            .child(format!("{} · {}", status.title(), cards.len()))
                            .when(cards.is_empty(), |column| column.child("No tasks yet"))
                            .children(cards.into_iter().map(|card| {
                                let id = card.id;
                                v_flex()
                                    .gap_2()
                                    .p_3()
                                    .bg(cx.theme().secondary)
                                    .rounded_md()
                                    .child(card.title.clone())
                                    .child(if card.assignee.is_empty() {
                                        "Unassigned".to_owned()
                                    } else {
                                        format!("Owner: {}", card.assignee)
                                    })
                                    .child(
                                        Button::new(("advance", id))
                                            .label(format!("Move to {}", status.next().title()))
                                            .on_click(cx.listener(move |_, _, _, cx| {
                                                state::update(cx, |workspace| {
                                                    workspace.board.advance(id)
                                                });
                                            })),
                                    )
                                    .child(
                                        Button::new(("remove", id)).label("Delete task").on_click(
                                            cx.listener(move |_, _, _, cx| {
                                                state::update(cx, |workspace| {
                                                    workspace.board.remove(id)
                                                });
                                            }),
                                        ),
                                    )
                            }))
                    })),
            )
    }

    fn chat(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_4()
            .child(div().text_xl().child("AI chat"))
            .child(self.provider.label())
            .child("Chat is session-only. Board contents are never sent automatically.")
            .children(
                self.chat
                    .messages
                    .iter()
                    .enumerate()
                    .map(|(index, message)| {
                        v_flex()
                            .id(("message", index))
                            .gap_2()
                            .p_3()
                            .rounded_md()
                            .bg(cx.theme().secondary)
                            .child(message.role.title())
                            .child(message.content.clone())
                    }),
            )
            .children(self.chat.error.clone().map(|error| div().child(error)))
            .when(self.chat.pending, |view| {
                view.child("Waiting for assistant…")
            })
            .child(Input::new(&self.prompt).disabled(self.chat.pending))
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("send")
                            .primary()
                            .label("Send")
                            .disabled(self.chat.pending)
                            .on_click(cx.listener(|this, _, _, cx| this.send(cx))),
                    )
                    .child(
                        Button::new("clear-chat")
                            .label("Clear chat")
                            .disabled(self.chat.pending)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.chat = Chat::default();
                                this.prompt
                                    .update(cx, |input, cx| input.set_value("", window, cx));
                                cx.notify();
                            })),
                    ),
            )
            .when(
                matches!(
                    self.chat.messages.last().map(|message| &message.role),
                    Some(Role::Assistant)
                ),
                |view| view.child("Edit the draft above to send your next message."),
            )
    }
}

impl Render for AppRoot {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = cx.global::<AppState>();
        let route = state.workspace.route;
        let warning = state.warning.clone();
        let save_error = state.save_error.clone();
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(
                h_flex()
                    .p_4()
                    .gap_3()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(div().flex_1().child("Notion GPUI"))
                    .child(
                        Button::new("board")
                            .label("Team board")
                            .disabled(route == Route::Board)
                            .on_click(cx.listener(|_, _, _, cx| {
                                state::update(cx, |workspace| workspace.route = Route::Board);
                            })),
                    )
                    .child(
                        Button::new("chat")
                            .label("AI chat")
                            .disabled(route == Route::Chat)
                            .on_click(cx.listener(|_, _, _, cx| {
                                state::update(cx, |workspace| workspace.route = Route::Chat);
                            })),
                    ),
            )
            .children(warning.map(|warning| div().p_3().child(warning)))
            .children(save_error.map(|error| div().p_3().child(error)))
            .child(
                div()
                    .id("page")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .p_5()
                    .child(match route {
                        Route::Board => self.board(cx).into_any_element(),
                        Route::Chat => self.chat(cx).into_any_element(),
                    }),
            )
    }
}
