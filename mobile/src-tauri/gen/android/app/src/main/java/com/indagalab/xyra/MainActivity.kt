package com.indagalab.xyra

import android.os.Bundle
import android.webkit.JavascriptInterface
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge
import com.google.mlkit.vision.barcode.common.Barcode
import com.google.mlkit.vision.codescanner.GmsBarcodeScannerOptions
import com.google.mlkit.vision.codescanner.GmsBarcodeScanning
import org.json.JSONObject

class MainActivity : TauriActivity() {
  private var content: WebView? = null

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
  }

  override fun onWebViewCreate(webView: WebView) {
    content = webView
    webView.addJavascriptInterface(QrBridge(), "XyraQr")
  }

  /** Google's QR scanner, which needs no camera permission; the page gets the result in window.__xyraQrResult. */
  private inner class QrBridge {
    @JavascriptInterface
    fun scan() {
      runOnUiThread {
        val options = GmsBarcodeScannerOptions.Builder().setBarcodeFormats(Barcode.FORMAT_QR_CODE).build()
        GmsBarcodeScanning.getClient(this@MainActivity, options)
          .startScan()
          .addOnSuccessListener { deliver(it.rawValue) }
          .addOnCanceledListener { deliver(null) }
          .addOnFailureListener { deliver(null) }
      }
    }
  }

  private fun deliver(raw: String?) {
    val payload = if (raw == null) "null" else JSONObject.quote(raw)
    runOnUiThread { content?.evaluateJavascript("window.__xyraQrResult && window.__xyraQrResult($payload)", null) }
  }
}
