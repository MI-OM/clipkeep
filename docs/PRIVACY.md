# ClipKeep privacy notice

**Last updated:** 2026-10-27

**Draft for product disclosure; review before public release.** This notice describes the current application design and should be checked against the actual release build and applicable privacy laws.

## What ClipKeep stores on this device

- Download history in the app's local web storage: title, format, quality, job status, timestamps, output path, and error details. The submitted source URL is not a history field.
- Preferences in local web storage, including the chosen save folder when “Remember this folder” is enabled.
- Media files in the folder you choose.
- Managed media tools and speech model files under ClipKeep's app-local data folder after you request their installation.
- Temporary media and transcription data while a job runs. Temporary job data is intended to be removed when processing finishes; review the release implementation and test cleanup behavior.

Local storage is available to processes running under your Windows account. Anyone with access to your Windows account or files may be able to access the saved history, preferences, media, tools, and models. Clearing app history does not delete media files.

## Network connections

ClipKeep does not operate an account service or ClipKeep cloud-processing service and the app currently has no analytics or advertising code. However, network traffic does occur:

- When you preview or download a link, ClipKeep's locally managed yt-dlp process contacts the source platform and its content-delivery hosts. The URL and normal request/network metadata are visible to those services. ClipKeep does not provide cookies, passwords, or account sign-in automation.
- When you choose to install media tools, ClipKeep contacts the pinned tool publishers and download hosts and checks downloaded files against pinned digests.
- When you choose to install a transcription model, ClipKeep downloads model files from the configured model host. Transcription of your selected media is performed locally after setup; the media file itself is not uploaded by ClipKeep for transcription.
- Windows, your network provider, and security software may perform their own network or diagnostic activity under their own settings.

Do not use ClipKeep with private links or sensitive URLs unless you understand that the platform receives the request. The app does not intentionally send download history or local media to a ClipKeep server.

## User responsibility and platform permissions

You are responsible for choosing content and confirming you have the rights, permission, and platform authorization required to download and keep it. A public or technically accessible link is not proof of permission. Respect copyright, privacy, creator settings, platform terms, and applicable law. ClipKeep does not bypass DRM, login restrictions, private-content controls, or platform-imposed download restrictions.

This notice is not a contract or a liability waiver. A statement that the developer is “not liable for what people download” cannot remove rights or obligations imposed by applicable law. Any limitation of liability or user indemnity needs separate, jurisdiction-specific terms reviewed by qualified counsel before release.

## Contact and changes

For privacy questions, contact [omfmail@aol.com](mailto:omfmail@aol.com). This notice was last updated on 2026-10-27. Update it when storage, network behavior, third-party tools, or model providers change.
