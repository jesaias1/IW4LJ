package dev.iw4j.iw4l;

import android.app.Activity;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.Context;
import android.content.Intent;
import android.graphics.Color;
import android.graphics.Typeface;
import android.os.Build;
import android.os.Bundle;
import android.view.Gravity;
import android.view.ViewGroup;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;
import android.widget.Toast;

import java.io.BufferedReader;
import java.io.File;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.util.ArrayDeque;

/**
 * The app's front door. It starts the game, unless the last start never got
 * as far as being paused (a crash during startup), in which case it shows
 * what is known about why instead of starting a game that will die again:
 * the panic the native code saved, the system log of the failed run, whether
 * the game files are where the app looks for them, and the device.
 */
public class LaunchActivity extends Activity {
    static final String STATE_FILE = "launch_state.txt";
    static final String CRASH_FILE = "crash.txt";

    static void writeState(Context context, String state) {
        try {
            Files.write(new File(context.getFilesDir(), STATE_FILE).toPath(),
                state.getBytes(StandardCharsets.UTF_8));
        } catch (Exception ignored) {
        }
    }

    private static String readFile(File file) {
        try {
            return new String(Files.readAllBytes(file.toPath()), StandardCharsets.UTF_8).trim();
        } catch (Exception error) {
            return "";
        }
    }

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        boolean crashed = readFile(new File(getFilesDir(), STATE_FILE)).equals("starting");
        if (!crashed) {
            startGame();
            return;
        }
        showReport(buildReport());
    }

    private void startGame() {
        writeState(this, "starting");
        new File(getFilesDir(), CRASH_FILE).delete();
        startActivity(new Intent(this, GameActivity.class));
        finish();
    }

    private String buildReport() {
        StringBuilder out = new StringBuilder();
        out.append("IW4LJ did not start last time.\n\n");
        out.append("== Game files ==\n").append(gameFiles()).append("\n");
        String panic = readFile(new File(getFilesDir(), CRASH_FILE));
        out.append("== Error saved by the app ==\n")
            .append(panic.isEmpty() ? "(none: the app was killed or crashed natively)" : panic)
            .append("\n\n");
        out.append("== System log of the failed run ==\n").append(systemLog()).append("\n");
        out.append("== Device ==\n")
            .append(Build.MANUFACTURER).append(' ').append(Build.MODEL)
            .append(" (").append(Build.DEVICE).append(")\n")
            .append("Android ").append(Build.VERSION.RELEASE)
            .append(" (API ").append(Build.VERSION.SDK_INT).append(")\n")
            .append("ABI ").append(String.join(",", Build.SUPPORTED_ABIS)).append('\n')
            .append("SoC ").append(Build.SOC_MANUFACTURER).append(' ').append(Build.SOC_MODEL).append('\n')
            .append("Hardware ").append(Build.HARDWARE).append('\n');
        return out.toString();
    }

    /** Whether the folder the app reads the game from holds one. */
    private String gameFiles() {
        StringBuilder out = new StringBuilder();
        boolean found = false;
        File[] media = getExternalMediaDirs();
        File[] roots = new File[] {
            media.length > 0 ? media[0] : null,
            getExternalFilesDir(null),
        };
        for (File root : roots) {
            if (root == null) {
                continue;
            }
            File mw2 = new File(root, "mw2");
            boolean zone = new File(mw2, "zone").isDirectory();
            boolean main = new File(mw2, "main").isDirectory();
            out.append(mw2.getAbsolutePath()).append(": ")
                .append(mw2.isDirectory() ? "folder exists" : "folder missing")
                .append(", zone ").append(zone ? "yes" : "NO")
                .append(", main ").append(main ? "yes" : "NO").append('\n');
            found |= zone && main;
        }
        if (!found) {
            File base = media.length > 0 && media[0] != null ? media[0] : getFilesDir();
            out.append("The MW2 files were NOT found. Copy your Modern Warfare 2 ")
                .append("multiplayer 'zone' and 'main' folders into a folder named 'mw2' at:\n")
                .append(base.getAbsolutePath()).append("/mw2/\n");
        } else {
            out.append("Game files found.\n");
        }
        return out.toString();
    }

    /** The warnings, errors and crash lines this app logged; an app may read its own log. */
    private String systemLog() {
        ArrayDeque<String> lines = new ArrayDeque<>();
        try {
            Process process = new ProcessBuilder("logcat", "-d", "-t", "1500", "-v", "brief")
                .redirectErrorStream(true).start();
            try (BufferedReader reader = new BufferedReader(
                new InputStreamReader(process.getInputStream(), StandardCharsets.UTF_8))) {
                String line;
                while ((line = reader.readLine()) != null) {
                    boolean keep = line.startsWith("E/") || line.startsWith("F/")
                        || line.contains("panicked") || line.contains("DEBUG")
                        || line.contains("Fatal signal") || line.contains("AndroidRuntime")
                        || line.contains("RustStdoutStderr") || line.contains("Vulkan")
                        || line.contains("wgpu") || line.contains("iw4");
                    if (keep) {
                        lines.addLast(line.length() > 300 ? line.substring(0, 300) : line);
                        if (lines.size() > 80) {
                            lines.removeFirst();
                        }
                    }
                }
            }
        } catch (Exception error) {
            return "(system log unavailable: " + error + ")\n";
        }
        if (lines.isEmpty()) {
            return "(nothing logged)\n";
        }
        return String.join("\n", lines) + "\n";
    }

    private void showReport(final String report) {
        int pad = (int) (16 * getResources().getDisplayMetrics().density);
        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setBackgroundColor(Color.BLACK);
        root.setPadding(pad, pad, pad, pad);

        TextView body = new TextView(this);
        body.setText(report);
        body.setTextColor(Color.WHITE);
        body.setTypeface(Typeface.MONOSPACE);
        body.setTextSize(11);
        body.setTextIsSelectable(true);
        ScrollView scroll = new ScrollView(this);
        scroll.addView(body);
        root.addView(scroll, new LinearLayout.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT, 0, 1f));

        LinearLayout buttons = new LinearLayout(this);
        buttons.setOrientation(LinearLayout.HORIZONTAL);
        buttons.setGravity(Gravity.CENTER);
        buttons.addView(button("Copy", v -> {
            ((ClipboardManager) getSystemService(CLIPBOARD_SERVICE))
                .setPrimaryClip(ClipData.newPlainText("IW4LJ report", report));
            Toast.makeText(this, "Copied", Toast.LENGTH_SHORT).show();
        }));
        buttons.addView(button("Share", v -> {
            Intent send = new Intent(Intent.ACTION_SEND).setType("text/plain")
                .putExtra(Intent.EXTRA_TEXT, report);
            startActivity(Intent.createChooser(send, "Share report"));
        }));
        buttons.addView(button("Try again", v -> startGame()));
        root.addView(buttons);
        setContentView(root);
    }

    private Button button(String label, android.view.View.OnClickListener listener) {
        Button button = new Button(this);
        button.setText(label);
        button.setOnClickListener(listener);
        button.setLayoutParams(new LinearLayout.LayoutParams(0,
            ViewGroup.LayoutParams.WRAP_CONTENT, 1f));
        return button;
    }
}
