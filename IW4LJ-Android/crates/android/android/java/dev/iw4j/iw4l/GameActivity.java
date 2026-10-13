package dev.iw4j.iw4l;

import android.app.ActivityManager;
import android.app.AlertDialog;
import android.os.Handler;
import android.os.Looper;
import android.text.InputFilter;
import android.text.InputType;
import android.widget.EditText;
import android.app.NativeActivity;
import android.content.Context;
import android.hardware.input.InputManager;
import android.net.wifi.WifiManager;
import android.os.Bundle;
import android.util.Log;
import android.util.SparseArray;
import android.view.InputDevice;
import android.view.KeyEvent;
import android.view.MotionEvent;
import android.view.View;
import android.view.WindowInsets;
import android.view.WindowInsetsController;
import android.view.WindowManager;

import java.io.File;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;

/**
 * The game's activity: a {@link NativeActivity} that keeps the input for itself.
 *
 * A NativeActivity hands its input queue to native code, where the windowing
 * library reads touches and discards what a controller says (its buttons become
 * unidentified keys and its sticks are dropped). So this activity takes the
 * queue back and forwards every touch, controller event and the Back key over
 * JNI, where they are replayed into the runtime's own input. Wired and
 * Bluetooth controllers arrive the same way: Android presents both as an
 * {@link InputDevice}.
 *
 * It also chooses the resolution the game is rendered at. The surface is given
 * a fixed size smaller than the screen and the display scales it up, which is
 * what lets a phone that cannot shade its full panel hold a frame rate.
 */
public class GameActivity extends NativeActivity implements InputManager.InputDeviceListener {
    static {
        // NativeActivity opens the library itself, outside the VM's own list,
        // so the native methods below would not resolve without this.
        System.loadLibrary("iw4l_android");
    }

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        // Cleared in onPause; if it is still set at the next launch, this one died.
        LaunchActivity.writeState(this, "starting");
        super.onCreate(savedInstanceState);
        // Input goes through the view hierarchy, to the dispatch methods below.
        getWindow().takeInputQueue(null);
        // A match has long stretches with no touch (a pad, a killcam); the
        // display must not time out under it.
        getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
        // Draw under the camera cutout: a game uses the whole panel.
        WindowManager.LayoutParams attributes = getWindow().getAttributes();
        attributes.layoutInDisplayCutoutMode =
            WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES;
        // A 120 Hz panel with a game that cannot reach 120 shows frames of
        // 8, 16, 25, 33 ms… in whatever mix the renderer manages. Asking for
        // 60 Hz gives the compositor a steady beat to hold the game to, runs
        // the panel and compositor at half the cost, and leaves the phone
        // cooler — so it can keep the clocks it needs.
        attributes.preferredRefreshRate = 120f;
        getWindow().setAttributes(attributes);
        // Development launches only (`am start ... --ez iw4l_dev true`): show
        // over the lock screen so a build can be tested on a device that locked
        // itself. A normal launch from the launcher never sets this.
        if (getIntent() != null && getIntent().getBooleanExtra("iw4l_dev", false)) {
            setShowWhenLocked(true);
            setTurnScreenOn(true);
        }
        getSystemService(InputManager.class).registerInputDeviceListener(this, null);
        // Wi-Fi drops broadcast packets for an app that has not asked for
        // them, and a LAN host is found by its broadcast.
        try {
            WifiManager wifi = (WifiManager) getApplicationContext().getSystemService(WIFI_SERVICE);
            broadcastLock = wifi.createMulticastLock("iw4lj-lan");
            broadcastLock.setReferenceCounted(false);
            broadcastLock.acquire();
        } catch (RuntimeException error) {
            Log.w("IW4LJ", "no Wi-Fi broadcast lock: " + error);
        }
        // The surface cannot be given a fixed size (a NativeActivity's is sized
        // by layout only), so the game is told the view's size and how many
        // lines to render, and creates a smaller swapchain the display scales.
        final int lines = renderHeight();
        findViewById(android.R.id.content).addOnLayoutChangeListener(
            (view, left, top, right, bottom, oldLeft, oldTop, oldRight, oldBottom) ->
                nativeView(right - left, bottom - top, lines));
    }

    private WifiManager.MulticastLock broadcastLock;

    @Override
    protected void onDestroy() {
        if (broadcastLock != null && broadcastLock.isHeld()) {
            broadcastLock.release();
        }
        getSystemService(InputManager.class).unregisterInputDeviceListener(this);
        super.onDestroy();
    }

    @Override
    public void onWindowFocusChanged(boolean hasFocus) {
        super.onWindowFocusChanged(hasFocus);
        if (hasFocus) {
            hideSystemBars();
            // Deliver controller and touch events the moment they arrive, not
            // batched to the next frame: that is up to a frame of lag saved.
            getWindow().getDecorView().requestUnbufferedDispatch(
                InputDevice.SOURCE_JOYSTICK | InputDevice.SOURCE_GAMEPAD
                    | InputDevice.SOURCE_TOUCHSCREEN);
        }
    }

    /** Immersive: the bars come back with a swipe from the edge, then leave. */
    private void hideSystemBars() {
        getWindow().setDecorFitsSystemWindows(false);
        WindowInsetsController controller = getWindow().getInsetsController();
        if (controller != null) {
            controller.hide(WindowInsets.Type.systemBars());
            controller.setSystemBarsBehavior(
                WindowInsetsController.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE);
        }
    }

    // -- Text prompts ---------------------------------------------------------

    private final Handler handler = new Handler(Looper.getMainLooper());
    /** Rumble has a thread of its own: the main thread is busy with input and layout. */
    private final android.os.HandlerThread rumbleThread = startRumbleThread();
    private final Handler rumbleHandler = new Handler(rumbleThread.getLooper());

    private static android.os.HandlerThread startRumbleThread() {
        android.os.HandlerThread thread = new android.os.HandlerThread("iw4l-rumble",
            android.os.Process.THREAD_PRIORITY_URGENT_DISPLAY);
        thread.start();
        return thread;
    }

    /**
     * The game cannot be typed into from a phone keyboard, so when its menu
     * wants text it raises a flag that is polled here, and the answer goes back
     * a character at a time.
     */
    private final Runnable promptPoll = new Runnable() {
        @Override
        public void run() {
            int kind = nativeTakePrompt();
            if (kind == 1) {
                askText(kind, "Player name", 15, false);
            } else if (kind == 2) {
                askText(kind, "Host IP address (shown on the host's LAN tab)", 15, true);
            }
            handler.postDelayed(this, 300);
        }
    };

    // -- Controller vibration ---------------------------------------------------

    /** The game's rumble requests, applied to the pad's motors thirty times a second. */
    private final Runnable rumblePoll = new Runnable() {
        @Override
        public void run() {
            try {
                if (nativeTakeRumble()) {
                    vibratePad(
                        (int) nativeRumbleField(0),
                        nativeRumbleField(1),
                        nativeRumbleField(2),
                        (int) nativeRumbleField(3));
                }
            } catch (RuntimeException error) {
                Log.w("IW4LPad", "vibration: " + error);
                rumbleDevice = -1;
            }
            rumbleHandler.postDelayed(this, 4);
        }
    };

    private static int amplitude(float level) {
        return Math.max(1, Math.min(255, Math.round(level * 255f)));
    }

    /** The motors of the pad last vibrated: looking them up costs a binder call each time. */
    private int rumbleDevice = -1;
    private android.os.VibratorManager rumbleManager;
    private int[] rumbleMotors = new int[0];

    private void vibratePad(int deviceId, float strong, float weak, int millis) {
        if (deviceId != rumbleDevice || rumbleManager == null) {
            InputDevice device = InputDevice.getDevice(deviceId);
            if (device == null) {
                return;
            }
            rumbleManager = device.getVibratorManager();
            rumbleMotors = rumbleManager.getVibratorIds();
            rumbleDevice = deviceId;
        }
        android.os.VibratorManager manager = rumbleManager;
        int[] motors = rumbleMotors;
        if (motors.length == 0) {
            return;
        }
        if ((strong <= 0f && weak <= 0f) || millis <= 0) {
            manager.cancel();
            return;
        }
        android.os.CombinedVibration.ParallelCombination both =
            android.os.CombinedVibration.startParallel();
        if (motors.length >= 2) {
            // Two motors: the first is the heavy one, the second the light one.
            if (strong > 0f) {
                both.addVibrator(motors[0],
                    android.os.VibrationEffect.createOneShot(millis, amplitude(strong)));
            }
            if (weak > 0f) {
                both.addVibrator(motors[1],
                    android.os.VibrationEffect.createOneShot(millis, amplitude(weak)));
            }
        } else {
            both.addVibrator(motors[0],
                android.os.VibrationEffect.createOneShot(millis, amplitude(Math.max(strong, weak))));
        }
        manager.vibrate(both.combine());
    }

    private void askText(final int kind, String title, int maxLength, boolean address) {
        final EditText input = new EditText(this);
        input.setSingleLine(true);
        if (address) {
            input.setInputType(InputType.TYPE_CLASS_NUMBER | InputType.TYPE_NUMBER_FLAG_DECIMAL);
            input.setKeyListener(android.text.method.DigitsKeyListener.getInstance("0123456789."));
        } else {
            input.setInputType(InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS);
        }
        input.setFilters(new InputFilter[] {new InputFilter.LengthFilter(maxLength)});
        new AlertDialog.Builder(this)
            .setTitle(title)
            .setView(input)
            .setPositiveButton(android.R.string.ok, (dialog, which) -> {
                String text = input.getText().toString();
                for (int index = 0; index < text.length(); ) {
                    int character = text.codePointAt(index);
                    nativeTypedChar(character);
                    index += Character.charCount(character);
                }
                nativeTypedDone(kind);
            })
            .setNegativeButton(android.R.string.cancel, null)
            .setOnDismissListener(dialog -> hideSystemBars())
            .show();
    }

    @Override
    protected void onResume() {
        super.onResume();
        handler.removeCallbacks(promptPoll);
        handler.post(promptPoll);
        rumbleHandler.removeCallbacks(rumblePoll);
        rumbleHandler.post(rumblePoll);
    }

    @Override
    protected void onPause() {
        handler.removeCallbacks(promptPoll);
        rumbleHandler.removeCallbacks(rumblePoll);
        releasePads();
        LaunchActivity.writeState(this, "ok");
        super.onPause();
    }

    // -- Render resolution ---------------------------------------------------

    /**
     * Lines to render. {@code render_height.txt} beside the game (written by
     * the mod menu) wins; otherwise the device's memory picks, as a stand-in
     * for how strong its GPU is.
     */
    private int renderHeight() {
        try {
            File file = new File(getExternalFilesDir(null), "render_height.txt");
            if (file.isFile()) {
                String text = new String(Files.readAllBytes(file.toPath()), StandardCharsets.UTF_8);
                return Math.max(360, Math.min(2160, Integer.parseInt(text.trim())));
            }
        } catch (Exception ignored) {
            // An unreadable setting is no setting.
        }
        ActivityManager.MemoryInfo memory = new ActivityManager.MemoryInfo();
        ((ActivityManager) getSystemService(Context.ACTIVITY_SERVICE)).getMemoryInfo(memory);
        long megabytes = memory.totalMem >> 20;
        if (megabytes >= 10500) {
            return 900;
        }
        return megabytes >= 6500 ? 720 : 540;
    }


    // -- Input ---------------------------------------------------------------

    @Override
    public boolean dispatchTouchEvent(MotionEvent event) {
        View content = findViewById(android.R.id.content);
        if (content == null || content.getWidth() == 0 || content.getHeight() == 0) {
            return true;
        }
        int[] origin = new int[2];
        content.getLocationInWindow(origin);
        float width = content.getWidth();
        float height = content.getHeight();
        int action = event.getActionMasked();
        switch (action) {
            case MotionEvent.ACTION_DOWN:
            case MotionEvent.ACTION_POINTER_DOWN:
            case MotionEvent.ACTION_UP:
            case MotionEvent.ACTION_POINTER_UP: {
                int index = event.getActionIndex();
                boolean down =
                    action == MotionEvent.ACTION_DOWN || action == MotionEvent.ACTION_POINTER_DOWN;
                nativeTouch(
                    event.getPointerId(index),
                    down ? 0 : 2,
                    (event.getX(index) - origin[0]) / width,
                    (event.getY(index) - origin[1]) / height);
                break;
            }
            case MotionEvent.ACTION_MOVE:
            case MotionEvent.ACTION_CANCEL: {
                for (int index = 0; index < event.getPointerCount(); index++) {
                    nativeTouch(
                        event.getPointerId(index),
                        action == MotionEvent.ACTION_MOVE ? 1 : 3,
                        (event.getX(index) - origin[0]) / width,
                        (event.getY(index) - origin[1]) / height);
                }
                break;
            }
            default:
                break;
        }
        return true;
    }

    private static boolean isPad(int source) {
        return (source & InputDevice.SOURCE_JOYSTICK) == InputDevice.SOURCE_JOYSTICK
            || (source & InputDevice.SOURCE_GAMEPAD) == InputDevice.SOURCE_GAMEPAD;
    }

    /** An axis centred on zero, with the device's own dead band removed. */
    private static float axis(MotionEvent event, int axis) {
        float value = event.getAxisValue(axis);
        InputDevice device = event.getDevice();
        InputDevice.MotionRange range =
            device == null ? null : device.getMotionRange(axis, event.getSource());
        if (range != null && Math.abs(value) <= range.getFlat()) {
            return 0f;
        }
        return value;
    }

    /** Which axes a pad's right stick and triggers are on. */
    private static final class PadLayout {
        int rightX = MotionEvent.AXIS_Z;
        int rightY = MotionEvent.AXIS_RZ;
        boolean triggersOnZ;
    }

    private final SparseArray<PadLayout> padLayouts = new SparseArray<>();

    /**
     * Pads come in two axis layouts and Android does not say which. One has
     * the right stick on Z/RZ and its triggers on axes of their own (the Xbox
     * and DualShock key layouts Android ships). The other is a plain HID pad in
     * XInput order — left stick, left trigger, right stick, right trigger — which
     * arrives as X/Y, Z, RX/RY, RZ with no trigger axes at all. Reading the
     * second as the first turns a resting trigger into a stick held hard over.
     */
    private PadLayout layoutOf(InputDevice device) {
        int id = device == null ? -1 : device.getId();
        PadLayout layout = padLayouts.get(id);
        if (layout != null) {
            return layout;
        }
        layout = new PadLayout();
        if (device != null) {
            boolean ownTriggers =
                has(device, MotionEvent.AXIS_LTRIGGER)
                    || has(device, MotionEvent.AXIS_RTRIGGER)
                    || has(device, MotionEvent.AXIS_BRAKE)
                    || has(device, MotionEvent.AXIS_GAS);
            boolean zPair = has(device, MotionEvent.AXIS_Z) && has(device, MotionEvent.AXIS_RZ);
            boolean rPair = has(device, MotionEvent.AXIS_RX) && has(device, MotionEvent.AXIS_RY);
            if (rPair && !(zPair && ownTriggers)) {
                layout.rightX = MotionEvent.AXIS_RX;
                layout.rightY = MotionEvent.AXIS_RY;
                layout.triggersOnZ = zPair && !ownTriggers;
            }
            StringBuilder axes = new StringBuilder();
            for (InputDevice.MotionRange range : device.getMotionRanges()) {
                axes.append(MotionEvent.axisToString(range.getAxis()))
                    .append('[').append(range.getMin()).append("..").append(range.getMax())
                    .append("] ");
            }
            Log.i("IW4LPad", "pad " + id + " '" + device.getName() + "' vendor=0x"
                + Integer.toHexString(device.getVendorId()) + " product=0x"
                + Integer.toHexString(device.getProductId()) + " right="
                + MotionEvent.axisToString(layout.rightX) + "/"
                + MotionEvent.axisToString(layout.rightY) + " triggersOnZ="
                + layout.triggersOnZ + " axes: " + axes);
        }
        padLayouts.put(id, layout);
        return layout;
    }

    private static boolean has(InputDevice device, int axis) {
        return device.getMotionRange(axis) != null;
    }

    /** A trigger axis as 0..1, wherever its range starts. */
    private static float unit(MotionEvent event, int axis) {
        InputDevice device = event.getDevice();
        InputDevice.MotionRange range = device == null ? null : device.getMotionRange(axis);
        float value = event.getAxisValue(axis);
        if (range == null || range.getRange() <= 0f) {
            return Math.max(0f, value);
        }
        float pulled = (value - range.getMin()) / range.getRange();
        return pulled < 0.06f ? 0f : Math.min(1f, pulled);
    }

    @Override
    public boolean dispatchGenericMotionEvent(MotionEvent event) {
        if (isPad(event.getSource()) && event.getActionMasked() == MotionEvent.ACTION_MOVE) {
            PadLayout layout = layoutOf(event.getDevice());
            float rx = axis(event, layout.rightX);
            float ry = axis(event, layout.rightY);
            float lt;
            float rt;
            if (layout.triggersOnZ) {
                lt = unit(event, MotionEvent.AXIS_Z);
                rt = unit(event, MotionEvent.AXIS_RZ);
            } else {
                // LTRIGGER/RTRIGGER on some pads, BRAKE/GAS on others.
                lt = Math.max(
                    axis(event, MotionEvent.AXIS_LTRIGGER), axis(event, MotionEvent.AXIS_BRAKE));
                rt = Math.max(
                    axis(event, MotionEvent.AXIS_RTRIGGER), axis(event, MotionEvent.AXIS_GAS));
            }
            PadState state = stateOf(event.getDeviceId());
            state.lx = axis(event, MotionEvent.AXIS_X);
            state.ly = axis(event, MotionEvent.AXIS_Y);
            state.rx = rx;
            state.ry = ry;
            state.lt = Math.max(0f, lt);
            state.rt = Math.max(0f, rt);
            state.hatX = event.getAxisValue(MotionEvent.AXIS_HAT_X);
            state.hatY = event.getAxisValue(MotionEvent.AXIS_HAT_Y);
            sendPad(event.getDeviceId(), state);
            return true;
        }
        return super.dispatchGenericMotionEvent(event);
    }

    /**
     * Everything one pad last said. A trigger is reported as an axis, as a key
     * (BUTTON_L2), or as both depending on the pad, and the two reports of one
     * pull disagree while it is under way; each of them used to be forwarded as
     * if it were a separate button, so the game saw the trigger flick on and
     * off. Here they are one value, the larger, and that is all that is sent.
     */
    private static final class PadState {
        float lx, ly, rx, ry, lt, rt, hatX, hatY;
        boolean l2Key, r2Key;
    }

    private final SparseArray<PadState> padStates = new SparseArray<>();

    private PadState stateOf(int device) {
        PadState state = padStates.get(device);
        if (state == null) {
            state = new PadState();
            padStates.put(device, state);
        }
        return state;
    }

    private void sendPad(int device, PadState state) {
        nativePadAxes(
            device,
            state.lx,
            state.ly,
            state.rx,
            state.ry,
            Math.max(state.lt, state.l2Key ? 1f : 0f),
            Math.max(state.rt, state.r2Key ? 1f : 0f),
            state.hatX,
            state.hatY);
    }

    /** Let go of everything: a release that happens while the game is away is never seen. */
    private void releasePads() {
        for (int index = 0; index < padStates.size(); index++) {
            int device = padStates.keyAt(index);
            padStates.put(device, new PadState());
            sendPad(device, padStates.valueAt(index));
        }
    }

    @Override
    public boolean dispatchKeyEvent(KeyEvent event) {
        int code = event.getKeyCode();
        boolean down = event.getAction() == KeyEvent.ACTION_DOWN;
        boolean padKey =
            KeyEvent.isGamepadButton(code)
                || (isPad(event.getSource())
                    && code >= KeyEvent.KEYCODE_DPAD_UP
                    && code <= KeyEvent.KEYCODE_DPAD_RIGHT);
        if (padKey) {
            if (event.getRepeatCount() == 0) {
                if (down) {
                    Log.d("IW4LPad", "button " + KeyEvent.keyCodeToString(code));
                }
                if (code == KeyEvent.KEYCODE_BUTTON_L2 || code == KeyEvent.KEYCODE_BUTTON_R2) {
                    // A trigger: merged with its axis, never sent as a button.
                    PadState state = stateOf(event.getDeviceId());
                    if (code == KeyEvent.KEYCODE_BUTTON_L2) {
                        state.l2Key = down;
                    } else {
                        state.r2Key = down;
                    }
                    sendPad(event.getDeviceId(), state);
                } else {
                    nativePadButton(event.getDeviceId(), code, down);
                }
            }
            return true;
        }
        if (code == KeyEvent.KEYCODE_BACK) {
            // Some pads send B as Back as well as BUTTON_B; the button is enough.
            if (down && event.getRepeatCount() == 0 && !isPad(event.getSource())) {
                nativeBack();
            }
            return true;
        }
        return super.dispatchKeyEvent(event);
    }

    @Override
    public void onInputDeviceAdded(int deviceId) {}

    @Override
    public void onInputDeviceChanged(int deviceId) {}

    @Override
    public void onInputDeviceRemoved(int deviceId) {
        padLayouts.remove(deviceId);
        padStates.remove(deviceId);
        nativePadRemoved(deviceId);
    }

    private static native void nativeTouch(int id, int phase, float x, float y);

    private static native void nativePadButton(int device, int keycode, boolean down);

    private static native void nativePadAxes(
        int device,
        float lx,
        float ly,
        float rx,
        float ry,
        float lt,
        float rt,
        float hatX,
        float hatY);

    private static native void nativePadRemoved(int device);

    private static native boolean nativeTakeRumble();

    private static native float nativeRumbleField(int index);

    private static native int nativeTakePrompt();

    private static native void nativeTypedChar(int character);

    private static native void nativeTypedDone(int kind);

    private static native void nativeBack();

    private static native void nativeView(int width, int height, int renderHeight);
}
