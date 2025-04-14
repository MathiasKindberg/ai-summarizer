# Hacker News AI news Summarizer

CLI tool utilizing the OpenAI API to score Hacker News stories based on their AI impact, summarize them and then posts the results to a Google Chat room.

## Usage

### Environment variables

Either copy the .env.example file to the directory where the summarizer runs or inject them.

### Example crontab to schedule running the summarizer every day at 9:00 UTC

```
0 9 * * * cd /root/ai-summarizer && ./ai-summarizer
```

### CLI arguments

```
-e, --export-text     Export the stories to json in the export directory
-r, --reset           Reset the database
-l, --log-to-console  Log to console
```
