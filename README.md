# <p align="center">GitHub Issue Tracker</p>

<p align="center">
  <a href="https://discord.gg/ccZn9ZMfFf">
    <img src="https://img.shields.io/badge/chat-Discord-7289DA?logo=discord" alt="flows.network Discord">
  </a>
  <a href="https://twitter.com/flows_network">
    <img src="https://img.shields.io/badge/Twitter-1DA1F2?logo=twitter&amp;logoColor=white" alt="flows.network Twitter">
  </a>
   <a href="https://flows.network/flow/createByTemplate/github-issue-notification-tracker">
    <img src="https://img.shields.io/website?up_message=deploy&url=https%3A%2F%2Fflows.network%2Fflow%2Fnew" alt="Create a flow">
  </a>
</p>

[Deploy this function on flows.network](#deploy-your-own-github-issue-tracker-bot-in-3-simple-steps), and you will get a Discord 🤖 to sync with the GitHub issues with specified labels. Label helps the open-source projects manage the GitHub issues. This bot can help DevRel and maintainers of open-source projects to interact with the community based on different labels automatically! For example, we can build a bot to send the GitHub issue with `good first issue` to the `contributor` channel on Discord based on this template.

<img width="800" alt="image" src="https://github.com/flows-network/issue-tracker/assets/45785633/2ebfc405-c99a-4703-95d9-13871c65b250">

The issues will appear as separate threads, and any updates made to the issue on GitHub, such as adding a new label or comment, will be synchronized with the corresponding thread on Discord.


## Deploy your own GitHub issue tracker bot in 3 simple steps

1. Create a bot from a template
2. Configure the bot on a Discord channel
3. Configure the bot to monitor the GitHub repo

### 0 Prerequisites

You will need to sign into [flows.network](https://flows.network/) from your GitHub account. It is free.

### 1 Create a bot from a template

[**Just click here**](https://flows.network/flow/createByTemplate/github-issue-notification-tracker)


Click on the **Create and Build** button.

### 2 Configure the bot to access Discord

You will now set up the Discord integration. Enter the `discord_channel_id` and `discord_token` to configure the bot. [Click here to learn how to get a Discord channel id and Discord bot token](https://flows.network/blog/discord-bot-guide).

* `discord_channel_id`: specify the channel where you wish to deploy the bot. You can copy and paste the final set of serial numbers from the URL.
* `discord_token`: get the Discord token from the Discord Developer Portal. This is standalone.

[<img width="450" alt="image" src="https://flows.network/assets/images/discord-flows-9036f93cbf0ab0ea6dbecca8ea17f84b.png">](https://flows.network/assets/images/discord-flows-9036f93cbf0ab0ea6dbecca8ea17f84b.png)


### 3 Configure the bot to access GitHub

Next, you will tell the bot which GitHub repo it needs to monitor for upcoming GitHub issues.

* `github_owner`: GitHub org for the repo *you want to deploy the 🤖 on*.
* `github_repo` : GitHub repo *you want to deploy the 🤖 on*.
* `labels`: the labels of GitHub issues that you want to monitor. Multiple labels are supported

> Let's see an example. You would like to deploy the bot to monitor the issues with `good first issue` and `help wanted` on `WasmEdge/docs` repo. Here `github_owner = WasmEdge`, `github_repo = docs` and `lables - good first issue, help wanted`.

[<img width="450" alt="image" src="https://github.com/flows-network/issue-tracker/assets/45785633/a30c1d30-b4ae-4d1e-8893-129d6165d133">](https://github.com/flows-network/issue-tracker/assets/45785633/a30c1d30-b4ae-4d1e-8893-129d6165d133)


Click on the **Connect** or **+ Add new authentication** button to give the function access to the GitHub repo to deploy the 🤖. You'll be redirected to a new page where you must grant [flows.network](https://flows.network/) permission to the repo.


[<img width="450" alt="image" src="https://github.com/flows-network/github-pr-summary/assets/45785633/6cefff19-9eeb-4533-a20b-03c6a9c89473">](https://github.com/flows-network/github-pr-summary/assets/45785633/6cefff19-9eeb-4533-a20b-03c6a9c89473)

Close the tab and go back to the flow.network page once you are done. Click on **Deploy**.

### Wait for the magic!

This is it! You are now on the flow details page waiting for the flow function to build. As soon as the flow's status became `running`, the bot is ready to track the GitHub issues! The bot is triggered by a GitHub issue labeled with specified labels.


