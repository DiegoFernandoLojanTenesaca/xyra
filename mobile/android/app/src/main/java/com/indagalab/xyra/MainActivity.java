package com.indagalab.xyra;

import android.app.Activity;
import android.app.AlertDialog;
import android.content.SharedPreferences;
import android.net.Uri;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.webkit.JavascriptInterface;
import android.webkit.WebView;
import android.webkit.WebViewClient;
import android.widget.Toast;
import com.google.mlkit.vision.barcode.common.Barcode;
import com.google.mlkit.vision.codescanner.GmsBarcodeScannerOptions;
import com.google.mlkit.vision.codescanner.GmsBarcodeScanning;
import java.net.HttpURLConnection;
import java.net.URL;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

/** Pairs with Xyra on the PC through its QR code and shows the page Xyra serves to phones. */
public class MainActivity extends Activity {
    private static final String PAIRING_SCHEME = "xyra";
    private static final String PAIRING_HOST = "pair";
    private static final String PREFERENCES = "pairing";
    private static final int REACH_TIMEOUT_MS = 2500;
    private static final int BACKGROUND = 0xFF060606;

    private final ExecutorService background = Executors.newSingleThreadExecutor();
    private final Handler main = new Handler(Looper.getMainLooper());
    private WebView web;

    @Override
    protected void onCreate(Bundle state) {
        super.onCreate(state);
        web = new WebView(this);
        web.setBackgroundColor(BACKGROUND);
        web.getSettings().setJavaScriptEnabled(true);
        web.getSettings().setDomStorageEnabled(true);
        web.setWebViewClient(new WebViewClient());
        web.addJavascriptInterface(new Bridge(), "XyraApp");
        setContentView(web);
        if (pairing().getString("token", "").isEmpty()) {
            scan();
        } else {
            connect();
        }
    }

    @Override
    protected void onDestroy() {
        background.shutdownNow();
        super.onDestroy();
    }

    private SharedPreferences pairing() {
        return getSharedPreferences(PREFERENCES, MODE_PRIVATE);
    }

    private void scan() {
        GmsBarcodeScannerOptions options = new GmsBarcodeScannerOptions.Builder().setBarcodeFormats(Barcode.FORMAT_QR_CODE).build();
        GmsBarcodeScanning.getClient(this, options)
                .startScan()
                .addOnSuccessListener(code -> pair(code.getRawValue()))
                .addOnCanceledListener(this::connectIfPaired)
                .addOnFailureListener(error -> connectIfPaired());
    }

    private void connectIfPaired() {
        if (pairing().getString("token", "").isEmpty()) {
            finish();
        } else {
            connect();
        }
    }

    /** Keeps the addresses, port and token of a code like xyra://pair?hosts=a,b&port=47811&token=…. */
    private void pair(String code) {
        Uri uri = Uri.parse(code == null ? "" : code);
        String hosts = uri.getQueryParameter("hosts");
        String port = uri.getQueryParameter("port");
        String token = uri.getQueryParameter("token");
        if (!PAIRING_SCHEME.equals(uri.getScheme()) || !PAIRING_HOST.equals(uri.getHost()) || hosts == null || port == null || token == null) {
            Toast.makeText(this, R.string.invalid_code, Toast.LENGTH_LONG).show();
            connectIfPaired();
            return;
        }
        pairing().edit().putString("hosts", hosts).putString("port", port).putString("token", token).apply();
        connect();
    }

    /** Opens the first address of the PC that answers, trying the home network before Tailscale. */
    private void connect() {
        web.loadData("<body style='background:#060606;color:#8a8a90;font-family:sans-serif;padding:24px'>" + getString(R.string.looking) + "</body>", "text/html; charset=utf-8", "UTF-8");
        SharedPreferences saved = pairing();
        String port = saved.getString("port", "");
        String token = saved.getString("token", "");
        String[] hosts = saved.getString("hosts", "").split(",");
        background.execute(() -> {
            for (String host : hosts) {
                String base = "http://" + host + ":" + port;
                if (answers(base + "/api/state?t=" + Uri.encode(token))) {
                    main.post(() -> web.loadUrl(base + "/?t=" + Uri.encode(token)));
                    return;
                }
            }
            main.post(this::notFound);
        });
    }

    private static boolean answers(String address) {
        try {
            HttpURLConnection connection = (HttpURLConnection) new URL(address).openConnection();
            connection.setConnectTimeout(REACH_TIMEOUT_MS);
            connection.setReadTimeout(REACH_TIMEOUT_MS);
            int status = connection.getResponseCode();
            connection.disconnect();
            return status == HttpURLConnection.HTTP_OK;
        } catch (Exception unreachable) {
            return false;
        }
    }

    private void notFound() {
        if (isFinishing()) {
            return;
        }
        new AlertDialog.Builder(this)
                .setTitle(R.string.not_found_title)
                .setMessage(R.string.not_found)
                .setCancelable(false)
                .setPositiveButton(R.string.retry, (dialog, which) -> connect())
                .setNegativeButton(R.string.scan, (dialog, which) -> scan())
                .show();
    }

    @Override
    public void onBackPressed() {
        if (web.canGoBack()) {
            web.goBack();
        } else {
            super.onBackPressed();
        }
    }

    /** Lets the page ask to scan a new code, when the PC changed it. */
    private class Bridge {
        @JavascriptInterface
        public void scan() {
            main.post(MainActivity.this::scan);
        }
    }
}
