param(
    [string]$Title = "Xavier Brain 2",
    [string]$Message = "Notificación del motor de conocimiento"
)

try {
    [Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] | Out-Null
    [Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom.XmlDocument, ContentType = WindowsRuntime] | Out-Null

    $template = [Windows.UI.Notifications.ToastNotificationManager]::GetTemplateContent([Windows.UI.Notifications.ToastTemplateType]::ToastText02)
    $xmlDoc = [xml]$template.GetXml()
    $nodes = $xmlDoc.GetElementsByTagName("text")
    $nodes[0].InnerText = $Title
    $nodes[1].InnerText = $Message

    $winXml = New-Object Windows.Data.Xml.Dom.XmlDocument
    $winXml.LoadXml($xmlDoc.OuterXml)

    $toast = [Windows.UI.Notifications.ToastNotification]::new($winXml)
    [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier("Xavier Brain 2").Show($toast)
} catch {
    Write-Warning "No se pudo mostrar la notificación de Windows: $_"
}
