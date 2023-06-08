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

    let login = env::var("login").unwrap_or("jetjinser".to_string());
    let owner = env::var("owner").unwrap_or("jetjinser".to_string());
    let repo = env::var("repo").unwrap_or("fot".to_string());

    let token = env::var("discord_token").unwrap();
    let channel_id = env::var("channel_id").unwrap().parse().unwrap();

    let labels = env::var("labels").unwrap();

    let x_labels: HashSet<String> = labels.split_whitespace().map(|s| s.to_string()).collect();
    let events = vec!["issues", "issue_comment"];
    let gh_login = GithubLogin::Provided(login);

    let discord = HttpBuilder::new(token).build();

    let state = App {
        discord,
        x_labels,
        channel_id,
    };

    listen_to_event(&gh_login, &owner, &repo, events, |payload| {
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

impl App {
    async fn handle_issue(&self, iep: Box<IssuesEventPayload>) {
        let labels = iep.issue.labels;
        if labels.is_empty() {
            log::debug!("issue `{}` has no label", iep.issue.title);
            return;
        }

        let labels = HashSet::from_iter(labels.iter().map(|lb| lb.name.clone()));

        let labeled_in_x = labels.intersection(&self.x_labels).count();

        if labeled_in_x != 0 {
            match iep.action {
                IssuesEventAction::Opened => {
                    let Some(content) = iep.issue.body else {
                        log::warn!("issue `{}` has no body", iep.issue.title);
                        return;
                    };
                    let mid = self.send_msg(self.channel_id, content).await.unwrap();

                    store::set(&format!("{}:message", iep.issue.id), mid.into(), None);

                    let title = format!("{}#{}", iep.issue.title, iep.issue.number);
                    let cid = self.start_thread(mid, title).await.unwrap();

                    store::set(&format!("{}:channel", iep.issue.id), cid.into(), None);
                }
                IssuesEventAction::Closed => {
                    let thread_channel_id = store::get(&format!("{}:channel", iep.issue.id));
                    if let Some(cid) = thread_channel_id {
                        let cid = cid.as_u64().unwrap();

                        let title = iep.issue.title;
                        let name = format!("{}(closed)", title);

                        self.edit_thread(name, cid).await;
                    }
                }
                IssuesEventAction::Reopened => {
                    let thread_channel_id = store::get(&format!("{}:channel", iep.issue.id));
                    if let Some(cid) = thread_channel_id {
                        let cid = cid.as_u64().unwrap();

                        let title = iep.issue.title;

                        self.edit_thread(title, cid).await;
                    }
                }
                IssuesEventAction::Edited => {
                    let issue_id = iep.issue.id;
                    let channel_id = store::get(&format!("{}:channel", issue_id));
                    let message_id = store::get(&format!("{}:message", issue_id));
                    if let (Some(cid), Some(mid)) = (channel_id, message_id) {
                        let cid = cid.as_u64().unwrap();
                        let mid = mid.as_u64().unwrap();

                        let title = iep.issue.title;

                        self.edit_msg(cid, mid, title).await;
                    }
                }
                IssuesEventAction::Assigned => {
                    let channel_id = store::get(&format!("{}:channel", iep.issue.id));

                    if let Some(cid) = channel_id {
                        let cid = cid.as_u64().unwrap();

                        let assignee = iep.assignee.unwrap().login;
                        self.send_msg(cid, format!("Assigned: {}", assignee)).await;
                    }
                }
                IssuesEventAction::Unassigned => {
                    let channel_id = store::get(&format!("{}:channel", iep.issue.id));

                    if let Some(cid) = channel_id {
                        let cid = cid.as_u64().unwrap();

                        let assignee = iep.assignee.unwrap().login;
                        self.send_msg(cid, format!("Unassigned: {}", assignee))
                            .await;
                    }
                }
                IssuesEventAction::Labeled => {
                    let channel_id = store::get(&format!("{}:channel", iep.issue.id));

                    if let Some(cid) = channel_id {
                        let cid = cid.as_u64().unwrap();

                        let label = iep.label.unwrap().name;
                        self.send_msg(cid, format!("Labeled: {}", label)).await;
                    }
                }
                IssuesEventAction::Unlabeled => {
                    let channel_id = store::get(&format!("{}:channel", iep.issue.id));

                    if let Some(cid) = channel_id {
                        let cid = cid.as_u64().unwrap();

                        let label = iep.label.unwrap().name;
                        self.send_msg(cid, format!("Unlabeled: {}", label)).await;
                    }
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

    async fn send_msg(&self, channel_id: u64, content: String) -> Option<u64> {
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
            Ok(msg) => Some(msg.id.0),
            Err(e) => {
                log::warn!("failed to send message: {}", e);
                None
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
            Ok(gc) => Some(gc.id.0),
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
}

impl App {
    async fn handle_issue_comment(&self, icep: Box<IssueCommentEventPayload>) {
        match icep.action {
            IssueCommentEventAction::Created => {
                let issue_id = icep.issue.id;
                let channel_id = store::get(&format!("{}:channel", issue_id));
                if let Some(cid) = channel_id {
                    let cid = cid.as_u64().unwrap();

                    let title = icep.comment.body.unwrap_or("...".to_string());

                    self.send_msg(cid, title).await;
                }
            }
            IssueCommentEventAction::Deleted => {
                let issue_id = icep.issue.id;
                let channel_id = store::get(&format!("{}:channel", issue_id));
                let message_id = store::get(&format!("{}:message", issue_id));
                if let (Some(cid), Some(mid)) = (channel_id, message_id) {
                    let cid = cid.as_u64().unwrap();
                    let mid = mid.as_u64().unwrap();

                    self.del_msg(cid, mid).await;
                }
            }
            IssueCommentEventAction::Edited => {
                let issue_id = icep.issue.id;
                let channel_id = store::get(&format!("{}:channel", issue_id));
                let message_id = store::get(&format!("{}:message", issue_id));
                if let (Some(cid), Some(mid)) = (channel_id, message_id) {
                    let cid = cid.as_u64().unwrap();
                    let mid = mid.as_u64().unwrap();

                    let body = icep.comment.body.unwrap_or("...".to_string());

                    self.edit_msg(cid, mid, body).await;
                }
            }
            action => {
                log::info!("uncovered action: {:?}", action)
            }
        }
    }

    async fn del_msg(&self, channel_id: u64, message_id: u64) {
        let res = self.discord.delete_message(channel_id, message_id).await;
        if let Err(e) = res {
            log::warn!("failed to delete message: {}", e);
        }
    }
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
