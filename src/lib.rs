use std::{collections::HashSet, env};

use discord_flows::http::{Http, HttpBuilder};
use flowsnet_platform_sdk::logger;
use github_flows::{
    listen_to_event,
    octocrab::models::events::payload::{
        IssueCommentEventAction, IssueCommentEventPayload, IssuesEventAction, IssuesEventPayload,
    },
    EventPayload, GithubLogin,
};
use store_flows as store;

struct App {
    discord: Http,
    x_labels: HashSet<String>,
    channel_id: u64,
}

#[no_mangle]
#[tokio::main(flavor = "current_thread")]
pub async fn run() {
    logger::init();

    let login = env::var("login")
        .map(GithubLogin::Provided)
        .unwrap_or(GithubLogin::Default);
    let owner = env::var("github_owner").unwrap_or("jetjinser".to_string());
    let repo = env::var("github_repo").unwrap_or("fot".to_string());

    let token = env::var("discord_token").unwrap();
    let channel_id = env::var("discord_channel_id").unwrap().parse().unwrap();

    let labels = env::var("labels").unwrap();

    let x_labels: HashSet<String> = labels
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let events = vec!["issues", "issue_comment"];

    let discord = HttpBuilder::new(token).build();

    log::info!("Running flow with x_labels: {:?}", &x_labels);

    let state = App {
        discord,
        x_labels,
        channel_id,
    };

    listen_to_event(&login, &owner, &repo, events, |payload| {
        handle(payload, state)
    })
    .await;
}

async fn handle(payload: EventPayload, app: App) {
    match payload {
        EventPayload::IssuesEvent(iep) => {
            app.handle_issue(iep).await;
        }
        EventPayload::IssueCommentEvent(icep) => {
            app.handle_issue_comment(icep).await;
        }
        e => {
            log::info!("uncovered event: {}", payload_name(e))
        }
    }
}

// handle_issue {{{
impl App {
    async fn handle_issue(&self, iep: Box<IssuesEventPayload>) {
        let labels = iep.issue.labels;
        if labels.is_empty() {
            log::debug!("issue `{}` has no label", iep.issue.title);
            return;
        }
        log::debug!("issue `{}` labels: {:?}", iep.issue.title, labels);

        let labels = HashSet::from_iter(labels.iter().map(|lb| lb.name.clone()));
        let labelled_in_x = labels.intersection(&self.x_labels);
        let labelled: Vec<&String> = labelled_in_x.collect();

        let labelled_msg = labelled
            .iter()
            .map(|label| format!("`{}`", label))
            .collect::<Vec<String>>()
            .join(", ");
        log::debug!("labelled_msg: {}", &labelled_msg);

        if !labelled.is_empty() {
            match iep.action {
                IssuesEventAction::Closed => {
                    let thread_channel_id = store::get(&format!("{}:channel", iep.issue.id));
                    if let Some(cid) = thread_channel_id {
                        let cid = cid.as_str().unwrap().parse().unwrap();

                        let title = iep.issue.title;
                        let name = format!("{}(closed)", title);

                        self.edit_thread(name, cid).await;
                    } else {
                        log::warn!("failed to get channel_id");
                    }

                    log::debug!("Closed action done");
                }
                IssuesEventAction::Reopened => {
                    let thread_channel_id = store::get(&format!("{}:channel", iep.issue.id));
                    if let Some(cid) = thread_channel_id {
                        let cid = cid.as_str().unwrap().parse().unwrap();

                        let title = iep.issue.title;

                        self.edit_thread(title, cid).await;
                    }

                    log::debug!("Reopened action done");
                }
                IssuesEventAction::Edited => {
                    let issue_id = iep.issue.id;
                    let channel_id = store::get(&format!("{}:channel", issue_id));
                    let message_id = store::get(&format!("{}:message", issue_id));
                    if let (Some(cid), Some(mid)) = (channel_id, message_id) {
                        let cid = cid.as_str().unwrap().parse().unwrap();
                        let mid = mid.as_str().unwrap().parse().unwrap();

                        let title = iep.issue.title;

                        self.edit_msg(cid, mid, title).await;
                    } else {
                        log::warn!("failed to get channel_id and message_id");
                    }

                    log::debug!("Edited action done");
                }
                IssuesEventAction::Assigned => {
                    let channel_id = store::get(&format!("{}:channel", iep.issue.id));

                    if let Some(cid) = channel_id {
                        let cid = cid.as_str().unwrap().parse().unwrap();

                        let assignee = iep.assignee.unwrap().login;
                        self.send_msg(cid, format!("Assigned: {}", assignee)).await;
                    }

                    log::debug!("Assigned action done");
                }
                IssuesEventAction::Unassigned => {
                    let channel_id = store::get(&format!("{}:channel", iep.issue.id));

                    if let Some(cid) = channel_id {
                        let cid = cid.as_str().unwrap().parse().unwrap();

                        let assignee = iep.assignee.unwrap().login;
                        self.send_msg(cid, format!("Unassigned: {}", assignee))
                            .await;
                    } else {
                        log::warn!("failed to get channel_id");
                    }

                    log::debug!("Unassigned action done");
                }
                IssuesEventAction::Labeled => {
                    let channel_id = store::get(&format!("{}:channel", iep.issue.id));

                    if let Some(cid) = channel_id {
                        let cid = cid.as_str().unwrap().parse().unwrap();

                        let label = iep.label.unwrap().name;
                        self.send_msg(cid, format!("Labeled: {}", label)).await;

                        log::debug!("Labeled action done");
                    } else {
                        let author = iep.issue.user.login;
                        let url = iep.issue.html_url;
                        let content = format!(
                            "An issue is labelled with {}, created by {}\n> {}",
                            labelled_msg, author, url
                        );
                        let mid = self.send_msg(self.channel_id, content).await;

                        let title = format!("{}#{}", iep.issue.title, iep.issue.number);
                        let cid = self.start_thread(mid, title).await.unwrap();

                        if self.join_thread(cid).await {
                            if let Some(body) = iep.issue.body {
                                self.send_msg(cid, body).await;
                            } else {
                                log::debug!("issue `{}` has no body", iep.issue.title);
                            }
                        }

                        {
                            store::set(
                                &format!("{}:message", iep.issue.id),
                                mid.to_string().into(),
                                None,
                            );

                            store::set(
                                &format!("{}:channel", iep.issue.id),
                                cid.to_string().into(),
                                None,
                            );
                        }

                        log::debug!(
                            "created and joined thread, stored message_id: {}, channel_id: {}",
                            mid,
                            cid
                        );
                    }
                    log::debug!("Labeled action done");
                }
                IssuesEventAction::Unlabeled => {
                    let channel_id = store::get(&format!("{}:channel", iep.issue.id));

                    if let Some(cid) = channel_id {
                        let cid = cid.as_str().unwrap().parse().unwrap();

                        let label = iep.label.unwrap().name;
                        self.send_msg(cid, format!("Unlabeled: {}", label)).await;
                    } else {
                        log::warn!("failed to get channel_id");
                    }

                    log::debug!("Unlabeled action done");
                }
                action => {
                    log::info!("uncovered issue action: {:?}", action)
                }
            }
        } else {
            log::debug!(
                "issue `{}`'s labels does not matching x_labels: {:?}",
                iep.issue.title,
                self.x_labels
            );
        }
    }
}
// }}}

// handle_issue_comment {{{
impl App {
    async fn handle_issue_comment(&self, icep: Box<IssueCommentEventPayload>) {
        match icep.action {
            IssueCommentEventAction::Created => {
                let issue_id = icep.issue.id;
                let channel_id = store::get(&format!("{}:channel", issue_id));
                if let Some(cid) = channel_id {
                    let cid = cid.as_str().unwrap().parse().unwrap();

                    let comment = icep.comment;
                    let author = comment.user.login;
                    let body = comment.body.unwrap_or("...".to_string());
                    let url = comment.html_url;

                    let content = format!("**{author}** added a *comment*:\n{body}\n\n> {url}");

                    let mid = self.send_msg(cid, content).await;

                    store::set(
                        &format!("{}:cmt_msg", comment.id),
                        mid.to_string().into(),
                        None,
                    );

                    log::debug!("stored comment_message_id: {}", mid);
                } else {
                    log::warn!("failed to get channel_id");
                }

                log::debug!("comment Created action done");
            }
            // IssueCommentEventAction::Deleted => {
            //     let issue_id = icep.issue.id;
            //     let channel_id = store::get(&format!("{}:channel", issue_id));
            //     let cmt_msg_id = store::get(&format!("{}:cmt_msg", issue_id));
            //     if let (Some(cid), Some(mid)) = (channel_id, cmt_msg_id) {
            //         let cid = cid.as_str().unwrap().parse().unwrap();
            //         let mid = mid.as_str().unwrap().parse().unwrap();
            //
            //         self.del_msg(cid, mid).await;
            //     } else {
            //         log::warn!("failed to get channel_id or comment_message_id");
            //     }
            //
            //     log::debug!("comment Deleted action done");
            // }
            // IssueCommentEventAction::Edited => {
            //     let issue_id = icep.issue.id;
            //     let channel_id = store::get(&format!("{}:channel", issue_id));
            //     let cmt_msg_id = store::get(&format!("{}:cmt_msg", issue_id));
            //     if let (Some(cid), Some(mid)) = (channel_id, cmt_msg_id) {
            //         let cid = cid.as_str().unwrap().parse().unwrap();
            //         let mid = mid.as_str().unwrap().parse().unwrap();
            //
            //         let body = icep.comment.body.unwrap_or("...".to_string());
            //
            //         self.edit_msg(cid, mid, body).await;
            //     } else {
            //         log::warn!("failed to get channel_id");
            //     }
            //
            //     log::debug!("comment Edited action done");
            // }
            action => {
                log::info!("uncovered action: {:?}", action)
            }
        }
    }
}
// }}}

// helper {{{
impl App {
    async fn send_msg(&self, channel_id: u64, content: String) -> u64 {
        let res = self
            .discord
            .send_message(
                channel_id,
                &serde_json::json!({
                    "content": content,
                }),
            )
            .await;

        match res {
            Ok(msg) => {
                log::debug!(
                    "Sended message {} to {}, message_id: {}",
                    content,
                    channel_id,
                    msg.id
                );
                msg.id.0
            }
            Err(e) => {
                log::warn!("failed to send message {} to {}", e, channel_id);
                panic!()
            }
        }
    }

    async fn start_thread(&self, mid: u64, title: String) -> Option<u64> {
        let mut map = serde_json::Map::new();
        map.insert("name".to_string(), title.into());
        let res = self
            .discord
            .create_public_thread(self.channel_id, mid, &map)
            .await;

        match res {
            Ok(gc) => {
                log::debug!("Started thread: {}({})", gc.name, gc.id);
                Some(gc.id.0)
            }
            Err(e) => {
                log::warn!("failed to creat public thread: {}", e);
                None
            }
        }
    }

    async fn edit_thread(&self, title: String, channel_id: u64) {
        let mut map = serde_json::Map::new();
        map.insert("name".to_string(), title.into());

        let res = self.discord.edit_thread(channel_id, &map).await;

        if let Err(e) = res {
            log::warn!("failed to edit channel: {}", e);
        }
    }

    async fn edit_msg(&self, channel_id: u64, message_id: u64, content: String) {
        let res = self
            .discord
            .edit_message(
                channel_id,
                message_id,
                &serde_json::json!({
                    "content": content
                }),
            )
            .await;

        if let Err(e) = res {
            log::warn!("failed to edit message: {}", e);
        }
    }

    async fn join_thread(&self, channel_id: u64) -> bool {
        let res = self.discord.join_thread_channel(channel_id).await;

        if let Err(e) = res {
            log::warn!("failed to join thread: {}", e);
            false
        } else {
            true
        }
    }

    // async fn del_msg(&self, channel_id: u64, message_id: u64) {
    //     let res = self.discord.delete_message(channel_id, message_id).await;
    //     if let Err(e) = res {
    //         log::warn!("failed to delete message: {}", e);
    //     }
    // }
}

fn payload_name(payload: EventPayload) -> &'static str {
    match payload {
        EventPayload::PushEvent(_) => "PushEvent",
        EventPayload::CreateEvent(_) => "CreateEvent",
        EventPayload::DeleteEvent(_) => "DeleteEvent",
        EventPayload::IssuesEvent(_) => "IssuesEvent",
        EventPayload::IssueCommentEvent(_) => "IssueCommentEvent",
        EventPayload::CommitCommentEvent(_) => "CommitCommentEvent",
        EventPayload::ForkEvent(_) => "ForkEvent",
        EventPayload::GollumEvent(_) => "GollumEvent",
        EventPayload::MemberEvent(_) => "MemberEvent",
        EventPayload::PullRequestEvent(_) => "PullRequestEvent",
        EventPayload::PullRequestReviewEvent(_) => "PullRequestReviewEvent",
        EventPayload::PullRequestReviewCommentEvent(_) => "PullRequestReviewCommentEvent",
        EventPayload::WorkflowRunEvent(_) => "WorkflowRunEvent",
        EventPayload::UnknownEvent(_) => "UnknownEvent",
        _ => "Unknown Unknown",
    }
}
// }}}
