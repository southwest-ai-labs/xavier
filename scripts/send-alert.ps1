$botToken = $env:TELEGRAM_BOT_TOKEN
$chatId = $env:TELEGRAM_CHAT_ID

if ([string]::IsNullOrWhiteSpace($botToken) -or [string]::IsNullOrWhiteSpace($chatId)) {
    Write-Error "TELEGRAM_BOT_TOKEN and TELEGRAM_CHAT_ID environment variables must be set."
    exit 1
}

$alertText = if ($args.Count -gt 0 -and -not [string]::IsNullOrWhiteSpace($args[0])) {
    $args[0]
} else {
    'Xavier Alert - Human review required.'
}

$body = @{
    chat_id = $chatId
    text    = $alertText
}

Invoke-RestMethod -Uri "https://api.telegram.org/bot$botToken/sendMessage" -Method Post -Body $body
