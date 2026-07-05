# Add project specific ProGuard rules here.
# You can control the set of applied configuration files using the
# proguardFiles setting in build.gradle.
#
# For more details, see
#   http://developer.android.com/guide/developing/tools/proguard.html

# If your project uses WebView with JS, uncomment the following
# and specify the fully qualified class name to the JavaScript interface
# class:
#-keepclassmembers class fqcn.of.javascript.interface.for.webview {
#   public *;
#}

# Uncomment this to preserve the line number information for
# debugging stack traces.
#-keepattributes SourceFile,LineNumberTable

# If you keep the line number information, uncomment this to
# hide the original source file name.
#-renamesourcefileattribute SourceFile

# -------------------------------------------------------
# JNA (Java Native Access) — required by UniFFI bindings
# The `peer` field in Pointer is accessed only from native
# JNI code in libjnidispatch.so; R8 must not strip it.
# -------------------------------------------------------
-keep class com.sun.jna.** { *; }
-keepclassmembers class com.sun.jna.** { *; }
-keep interface com.sun.jna.** { *; }
-keep class * implements com.sun.jna.Structure { *; }
-keep class * implements com.sun.jna.Callback { *; }

# JNA's Native$AWT references desktop java.awt classes that
# don't exist on Android — suppress the R8 missing-class error.
-dontwarn java.awt.**
-dontwarn sun.awt.**
-dontwarn java.applet.**

# UniFFI generated bindings
-keep class uniffi.** { *; }
-keepclassmembers class uniffi.** { *; }

