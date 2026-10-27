import {
  AudioLines,
  BadgeCheck,
  Clapperboard,
  Code2,
  Heart,
  ShieldCheck,
} from "lucide-react";

export function AboutPage() {
  return (
    <div className="content-page about-page">
      <div className="page-intro">
        <div>
          <p className="section-kicker">A LITTLE ABOUT US</p>
          <h1>About ClipKeep</h1>
          <p className="page-subtitle">A local desktop app for media you have permission to keep.</p>
        </div>
        <span className="page-icon-badge"><Clapperboard size={21} /></span>
      </div>

      <section className="about-hero-card">
        <div className="about-brand-mark"><Clapperboard size={28} /></div>
        <p className="about-version">CLIPKEEP · PREVIEW 0.1.0</p>
        <h2>Your media.<br /><span>Your computer.</span></h2>
        <p className="about-summary">
          ClipKeep processes downloads and transcriptions on your device. It has no ClipKeep account,
          analytics, or cloud conversion service. Platform and dependency downloads still connect to
          their respective providers.
        </p>
        <div className="about-pills">
          <span><ShieldCheck size={14} /> Local-first</span>
          <span><AudioLines size={14} /> MP4 & MP3</span>
          <span><Heart size={14} /> User-controlled</span>
        </div>
      </section>

      <section className="about-info-grid">
        <article className="about-info-card">
          <div className="about-info-icon"><BadgeCheck size={17} /></div>
          <div>
            <h3>Built with care</h3>
            <p>ClipKeep uses Tauri, React, TypeScript, and Rust. Real-media integration tests and release packaging are still in progress.</p>
          </div>
        </article>
        <article className="about-info-card">
          <div className="about-info-icon purple"><Code2 size={17} /></div>
          <div>
            <h3>Public release preparation</h3>
            <p>ClipKeep-authored source is Apache-2.0. Third-party notices and installer redistribution requirements are still under review.</p>
          </div>
        </article>
      </section>

      <div className="legal-note">
        <ShieldCheck size={17} />
        <p>
          You are responsible for confirming your rights and platform permission before downloading.
          A supported link does not mean its creator or platform permits a download. ClipKeep does not
          bypass DRM, private-content controls, or access restrictions. See the privacy notice for
          storage and network details. This is not a liability waiver or legal advice.
        </p>
      </div>
      <section className="privacy-notice" aria-labelledby="privacy-notice-title">
        <h2 id="privacy-notice-title">Privacy in plain language</h2>
        <p>Download history and preferences are stored on this device. ClipKeep has no account service, analytics, or advertising. Previewing/downloading sends the link to the source platform; installing tools or models contacts their publishers. Transcription runs locally after model setup. The selected media is not uploaded to ClipKeep.</p>
        <details>
          <summary>What is stored and sent?</summary>
          <p>Local history contains a title, format, quality, status, time, output path, and error details; it does not store the submitted URL as a history field. Preferences may include your save-folder path. Media, managed tools, and installed models stay on your device.</p>
          <p>For metadata and downloads, the source URL is requested from the platform and its delivery hosts, which receive ordinary connection information. Installing media tools or speech models contacts their publishers. ClipKeep does not send your media to a ClipKeep server for transcription. Windows and network software may have their own diagnostics.</p>
          <p>Local data is accessible to processes and people with access to your Windows account. Clearing download history does not delete saved media. Do not submit sensitive or private URLs unless you understand the platform will receive the request.</p>
          <p>This privacy summary is not a liability waiver. Any limitation of liability must be addressed in separate terms reviewed for applicable jurisdictions.</p>
        </details>
      </section>
    </div>
  );
}
