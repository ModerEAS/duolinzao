package com.duolinzao.app.ui

import android.app.Activity
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.duolinzao.app.api.ApiException
import com.duolinzao.app.api.Session
import com.duolinzao.app.auth.PasskeyHelper
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.serialization.json.Json

private enum class LoginScreenKind { Home, Methods, Password, Totp, Email, Wait }

/**
 * 登录版式对齐 Web LoginPage：首页 Passkey + 账号下一步；
 * methods 屏含「通行密钥」行；密码/TOTP/邮箱/新设备分屏。
 */
@Composable
fun LoginScreen(onDone: () -> Unit, onRegister: () -> Unit) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var screen by remember { mutableStateOf(LoginScreenKind.Home) }
    var username by remember { mutableStateOf("") }
    var password by remember { mutableStateOf("") }
    var totpCode by remember { mutableStateOf("") }
    var otpCode by remember { mutableStateOf("") }
    var otpSent by remember { mutableStateOf(false) }
    var waitInfo by remember { mutableStateOf("") }
    var error by remember { mutableStateOf<String?>(null) }
    var busy by remember { mutableStateOf(false) }
    var methods by remember { mutableStateOf(listOf("passkey", "totp", "password")) }
    var emailEnabled by remember { mutableStateOf(false) }

    LaunchedEffect(Unit) {
        try {
            val m = Session.repo.methods()
            methods = m.methods
            emailEnabled = m.email_enabled
        } catch (_: Exception) { /* 保持默认顺序 */ }
    }

    suspend fun enter() {
        Session.setMe(Session.repo.me())
        onDone()
    }

    fun fail(e: Exception) {
        error = if (e is ApiException) e.message else "请求失败"
    }

    fun runPasskey(named: Boolean) {
        scope.launch {
            busy = true
            error = null
            try {
                val activity = context as Activity
                val started = if (named) Session.repo.passkeyStart(username) else Session.repo.passkeyStart()
                val requestJson = started.publicKey?.let {
                    Json.encodeToString(kotlinx.serialization.json.JsonElement.serializer(), it)
                }.orEmpty()
                val assertion = PasskeyHelper.getAssertion(activity, requestJson)
                if (assertion == null) {
                    error = "通行密钥不可用，请用密码登录"
                    if (named || username.isNotBlank()) screen = LoginScreenKind.Password
                } else {
                    if (named) {
                        Session.repo.passkeyFinish(username = username, challengeId = started.challenge_id, credentialJson = assertion)
                    } else {
                        Session.repo.passkeyFinish(challengeId = started.challenge_id, credentialJson = assertion)
                    }
                    enter()
                }
            } catch (e: Exception) {
                fail(e)
                if (named || username.isNotBlank()) screen = LoginScreenKind.Password
            }
            busy = false
        }
    }

    Box(Modifier.fillMaxSize().background(DlBg).padding(24.dp), contentAlignment = Alignment.Center) {
        Column(
            Modifier
                .fillMaxWidth()
                .verticalScroll(rememberScrollState()),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Surface(
                shape = RoundedCornerShape(14.dp),
                color = DlCard,
                border = androidx.compose.foundation.BorderStroke(1.dp, ColorLine),
                shadowElevation = 1.dp,
                modifier = Modifier.fillMaxWidth().padding(bottom = 14.dp),
            ) {
                Column(Modifier.padding(horizontal = 24.dp, vertical = 18.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                        DlBrandMark()
                        Column {
                            Text("多邻灶", fontSize = 20.sp, fontWeight = FontWeight.Bold, color = DlInk)
                            Text("员工 / 食堂工作台", fontSize = 12.sp, color = DlGray)
                        }
                    }

                    when (screen) {
                        LoginScreenKind.Home -> {
                            Text("登录", fontSize = 22.sp, fontWeight = FontWeight.Bold, color = DlInk, textAlign = TextAlign.Center, modifier = Modifier.fillMaxWidth())
                            Text(
                                "使用通行密钥，或输入账号继续",
                                fontSize = 13.sp,
                                color = DlMuted,
                                textAlign = TextAlign.Center,
                                modifier = Modifier.fillMaxWidth().padding(bottom = 6.dp),
                            )
                            DlPrimaryButton("使用通行密钥登录", testTag = "login_passkey") { runPasskey(named = false) }
                            LoginDivider()
                            DlTextField(username, { username = it }, "账号", testTag = "login_username")
                            Row(
                                Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.SpaceBetween,
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                TextButton(onClick = onRegister, modifier = Modifier.testTag("login_register")) {
                                    Text("有邀请码，去注册", color = DlGreen)
                                }
                                androidx.compose.material3.Button(
                                    onClick = {
                                        if (username.isBlank()) {
                                            error = "请先填写账号"
                                        } else {
                                            error = null
                                            screen = LoginScreenKind.Methods
                                        }
                                    },
                                    colors = androidx.compose.material3.ButtonDefaults.buttonColors(containerColor = DlGreen),
                                    modifier = Modifier.testTag("login_next"),
                                ) { Text("下一步") }
                            }
                        }

                        LoginScreenKind.Methods -> {
                            Text("选择登录方式", fontSize = 22.sp, fontWeight = FontWeight.Bold, color = DlInk, textAlign = TextAlign.Center, modifier = Modifier.fillMaxWidth())
                            Box(Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) {
                                DlAccountChip(username)
                            }
                            DlMethodRow(
                                title = "通行密钥",
                                subtitle = "使用该账号绑定的 Passkey 登录",
                                onClick = { runPasskey(named = true) },
                                testTag = "login_method_passkey",
                            )
                            DlMethodRow(
                                title = "密码",
                                subtitle = "使用账号密码登录",
                                onClick = { screen = LoginScreenKind.Password },
                                testTag = "login_method_password",
                            )
                            if (methods.contains("totp")) {
                                DlMethodRow(
                                    title = "验证码",
                                    subtitle = "Authenticator 应用中的 6 位动态码",
                                    onClick = { screen = LoginScreenKind.Totp },
                                    testTag = "login_method_totp",
                                )
                            }
                            if (emailEnabled && methods.contains("email_otp")) {
                                DlMethodRow(
                                    title = "邮箱登录",
                                    subtitle = "向绑定邮箱发送验证码",
                                    onClick = { screen = LoginScreenKind.Email; otpSent = false },
                                    testTag = "login_method_email",
                                )
                            }
                            DlMethodRow(
                                title = "新设备确认",
                                subtitle = "请已登录设备在页内允许",
                                onClick = { screen = LoginScreenKind.Wait; waitInfo = "" },
                                testTag = "login_method_wait",
                            )
                            TextButton(onClick = { screen = LoginScreenKind.Home }, modifier = Modifier.fillMaxWidth()) {
                                Text("使用其他账号", color = DlMuted)
                            }
                        }

                        LoginScreenKind.Password -> {
                            Text("输入密码", fontSize = 22.sp, fontWeight = FontWeight.Bold, color = DlInk, textAlign = TextAlign.Center, modifier = Modifier.fillMaxWidth())
                            Box(Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) {
                                DlAccountChip(username)
                            }
                            DlTextField(password, { password = it }, "密码", password = true, testTag = "login_password")
                            DlPrimaryButton("登录", testTag = "login_submit") {
                                scope.launch {
                                    try {
                                        Session.repo.loginPassword(username, password)
                                        enter()
                                    } catch (e: Exception) { fail(e) }
                                }
                            }
                            TextButton(onClick = { screen = LoginScreenKind.Methods }, modifier = Modifier.fillMaxWidth()) {
                                Text("换一种方式", color = DlMuted)
                            }
                        }

                        LoginScreenKind.Totp -> {
                            Text("输入验证码", fontSize = 22.sp, fontWeight = FontWeight.Bold, color = DlInk, textAlign = TextAlign.Center, modifier = Modifier.fillMaxWidth())
                            Box(Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) {
                                DlAccountChip(username)
                            }
                            DlTextField(totpCode, { totpCode = it }, "6 位动态码", testTag = "login_totp")
                            Text("打开 Authenticator 应用查看验证码。", fontSize = 12.sp, color = DlGray, textAlign = TextAlign.Center, modifier = Modifier.fillMaxWidth())
                            DlPrimaryButton("验证并登录") {
                                scope.launch {
                                    try {
                                        Session.repo.loginTotp(username, totpCode)
                                        enter()
                                    } catch (e: Exception) { fail(e) }
                                }
                            }
                            TextButton(onClick = { screen = LoginScreenKind.Methods }, modifier = Modifier.fillMaxWidth()) {
                                Text("换一种方式", color = DlMuted)
                            }
                        }

                        LoginScreenKind.Email -> {
                            Text("邮箱登录", fontSize = 22.sp, fontWeight = FontWeight.Bold, color = DlInk, textAlign = TextAlign.Center, modifier = Modifier.fillMaxWidth())
                            Box(Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) {
                                DlAccountChip(username)
                            }
                            if (!otpSent) {
                                DlOutlineButton("发送验证码") {
                                    scope.launch {
                                        try {
                                            Session.repo.requestEmailOtp(username)
                                            otpSent = true
                                            error = "如果账号已绑定邮箱，验证码已发出"
                                        } catch (e: Exception) { fail(e) }
                                    }
                                }
                            }
                            DlTextField(otpCode, { otpCode = it }, "6 位验证码")
                            DlPrimaryButton("验证并登录") {
                                scope.launch {
                                    try {
                                        Session.repo.verifyEmailOtp(username, otpCode)
                                        enter()
                                    } catch (e: Exception) { fail(e) }
                                }
                            }
                            TextButton(onClick = { screen = LoginScreenKind.Methods }, modifier = Modifier.fillMaxWidth()) {
                                Text("换一种方式", color = DlMuted)
                            }
                        }

                        LoginScreenKind.Wait -> {
                            Text("等待设备确认", fontSize = 22.sp, fontWeight = FontWeight.Bold, color = DlInk, textAlign = TextAlign.Center, modifier = Modifier.fillMaxWidth())
                            Box(Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) {
                                DlAccountChip(username)
                            }
                            Text(
                                "已向你的已登录设备发起确认，请在对方页面允许或拒绝。",
                                fontSize = 13.sp,
                                color = DlMuted,
                                textAlign = TextAlign.Center,
                                modifier = Modifier.fillMaxWidth(),
                            )
                            DlOutlineButton("请已登录设备确认") {
                                scope.launch {
                                    try {
                                        val created = Session.repo.createApproval(username)
                                        waitInfo = "等待确认…"
                                        while (true) {
                                            delay(2000)
                                            val poll = Session.repo.pollApproval(created.id, created.poll_token)
                                            if (poll.status == "allowed") {
                                                waitInfo = "已通过"
                                                enter()
                                                break
                                            }
                                            if (poll.status != "pending") {
                                                waitInfo = "状态：${poll.status}"
                                                break
                                            }
                                        }
                                    } catch (e: Exception) { fail(e) }
                                }
                            }
                            if (waitInfo.isNotEmpty()) Text(waitInfo, color = DlMuted, fontSize = 12.sp)
                            TextButton(onClick = { screen = LoginScreenKind.Methods }, modifier = Modifier.fillMaxWidth()) {
                                Text("换一种方式", color = DlMuted)
                            }
                        }
                    }

                    ErrorLine(error)
                    if (busy) Text("处理中…", color = DlMuted, fontSize = 12.sp)
                }
            }
            Text("新设备将等待已登录设备页内确认 · 不扫码", fontSize = 12.sp, color = DlGray)
        }
    }
}

private val ColorLine = DlLine

@Composable
private fun LoginDivider() {
    Row(
        Modifier.fillMaxWidth().padding(vertical = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Box(Modifier.weight(1f).height(1.dp).background(Color(0xFFF0F1F3)))
        Text("或", fontSize = 12.sp, color = DlGray)
        Box(Modifier.weight(1f).height(1.dp).background(Color(0xFFF0F1F3)))
    }
}
