# RafLand

Dynamic Island untuk Windows bergaya kaca (glass) ala macOS. Dibangun di atas basis **azay**
(Rust + egui): animasi island, kontrol media, lirik, kecerahan/volume. Semua fitur lain ditulis ulang
dari nol (kode, teks, dan desain robot orisinal, tidak menyalin ReyLand).

## Cara dapat .exe dan installer tanpa Rust
1. Upload folder ini ke repo GitHub (private juga boleh).
2. Buka tab Actions, jalankan "Build RafLand" (otomatis tiap push).
3. Setelah hijau, unduh artifact `rafland-installer` (RafLand-Setup.exe) atau `rafland-exe`.
   Push tag `v0.1.0` untuk sekalian membuat Release.

## Build lokal (Windows)
1. Pasang Rust: https://rustup.rs
2. `cargo build --release`
3. Jalankan `target\release\rafland.exe`

## Fitur
- Island kaca: pill jam/baterai, pill musik (lirik berjalan), pill notifikasi, panel dengan bar ikon di bawah
  (Musik, Hari ini, Sholat, Shelf, Timer, Mata, Minum, Tangkap) dan ikon gigi untuk Pengaturan
- Robot kecil: berkedip, senang saat musik, kehausan saat pengingat minum
- Musik + lirik tersinkron (LRCLIB, dengan pencarian cadangan), seek, kecerahan, volume
- Mode rapat: deteksi Zoom/Meet/Teams lewat judul jendela (WhatsApp tidak dihitung), tombol mic bisa dimatikan
- Jadwal sholat: 35 kota Indonesia (Kemenag) + 56 kota Malaysia (sudut JAKIM; Zohor/Asar/Isyak), cuaca, adzan
  - Adzan: taruh `adzan.wav` di `%APPDATA%\RafLand\`; kalau tidak ada, bunyi sistem
  - Musik dijeda saat adzan, lalu island menawarkan melanjutkan
- Fokus: Pomodoro dan Timer, pengingat istirahat mata
- Minum: target harian, jam aktif, selang, klik notifikasi untuk mencatat (awalnya mati)
- Shelf: seret file ke island, riwayat clipboard (hanya di memori), kirim file ke HP lewat Wi-Fi yang sama
- Tangkap layar (pintasan bisa diganti, bawaan Ctrl+Shift+S) dan rekam layar (Win+Alt+R, Game Bar)
- Hari ini: kalender bulanan + acara/tugas per tanggal dengan notifikasi
- Sholat: grafik lengkung dengan penanda waktu sekarang dan lima kartu waktu
- Pengaturan dengan kotak cari (Ctrl+F)

## Belum ada / batasan
- Slider kecerahan dihapus dari tampilan baru (kode kontrolnya masih ada)
- Blur sungguhan (acrylic): kaca di sini tembus pandang + kilau, bukan blur latar
- Notifikasi aplikasi lain (mis. WhatsApp Web) belum dibaca; notifikasi yang ada berasal dari fitur RafLand sendiri
- Sembunyi saat slideshow/layar penuh dan pil kecil saat menutupi tombol belum dibuat
- Kirim ke HP tanpa QR (tautan ditampilkan sebagai teks); Windows akan meminta izin firewall
- Deteksi rapat di browser hanya bila tab rapat sedang aktif
- Kode belum pernah dikompilasi di sini; kemungkinan ada error kecil saat `cargo build` pertama
