package com.duolinzao.app.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

val DlGreen = Color(0xFF2D8A45)
val DlGreenSoft = Color(0xFFE8F5EC)
val DlBg = Color(0xFFF3F5F7)
val DlCard = Color(0xFFFFFFFF)
val DlInk = Color(0xFF1A1F24)
val DlMuted = Color(0xFF6B7280)
val DlGray = Color(0xFF9AA3AD)
val DlLine = Color(0xFFE5E8EC)
val DlDanger = Color(0xFFC0392B)

@Composable
fun DlTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = lightColorScheme(
            primary = DlGreen,
            secondary = DlGreen,
            background = DlBg,
            surface = DlCard,
            onPrimary = Color.White,
            onBackground = DlInk,
            onSurface = DlInk,
            onSurfaceVariant = DlMuted,
            outline = DlLine,
            error = DlDanger,
        ),
        content = content,
    )
}

@Composable
fun DlTextField(
    value: String,
    onValue: (String) -> Unit,
    label: String,
    password: Boolean = false,
    modifier: Modifier = Modifier,
    testTag: String = "",
) {
    OutlinedTextField(
        value = value,
        onValueChange = onValue,
        label = { Text(label) },
        singleLine = true,
        visualTransformation = if (password) PasswordVisualTransformation() else VisualTransformation.None,
        modifier = modifier.fillMaxWidth().then(if (testTag.isNotEmpty()) Modifier.testTag(testTag) else Modifier),
    )
}

@Composable
fun DlPrimaryButton(text: String, enabled: Boolean = true, testTag: String = "", onClick: () -> Unit) {
    Button(
        onClick = onClick,
        enabled = enabled,
        colors = ButtonDefaults.buttonColors(containerColor = DlGreen),
        modifier = Modifier.fillMaxWidth().then(if (testTag.isNotEmpty()) Modifier.testTag(testTag) else Modifier),
    ) { Text(text) }
}

@Composable
fun DlOutlineButton(text: String, enabled: Boolean = true, testTag: String = "", onClick: () -> Unit) {
    OutlinedButton(
        onClick = onClick,
        enabled = enabled,
        modifier = Modifier.fillMaxWidth().then(if (testTag.isNotEmpty()) Modifier.testTag(testTag) else Modifier),
    ) { Text(text) }
}

@Composable
fun ErrorLine(message: String?) {
    if (!message.isNullOrEmpty()) {
        Spacer(Modifier.height(8.dp))
        Text(message, color = Color(0xFFE53935), style = MaterialTheme.typography.bodySmall)
    }
}

/** 成功提示走绿色，别再用红色 ErrorLine 显示「已提交/已保存」。 */
@Composable
fun SuccessLine(message: String?) {
    if (!message.isNullOrEmpty()) {
        Spacer(Modifier.height(8.dp))
        Text(message, color = DlGreen, style = MaterialTheme.typography.bodySmall)
    }
}

@Composable
fun SectionTitle(text: String) {
    Text(text, style = MaterialTheme.typography.titleMedium, modifier = Modifier.padding(vertical = 8.dp))
}

/** 报餐色：绿=报，橙=不报（与 Web 预览一致）；locked 灰。 */
fun mealColor(color: String): Color = when (color) {
    "green" -> DlGreen
    "orange" -> Color(0xFFE6A23C)
    "locked" -> Color(0xFF9E9E9E)
    else -> Color(0xFFD1D5DB)
}

fun slotName(slot: String): String = when (slot) {
    "breakfast" -> "早餐"
    "lunch" -> "午餐"
    "dinner" -> "晚餐"
    else -> slot
}

/** 顶栏「灶」品牌标，对齐 Web / canteen-portrait-mock。 */
@Composable
fun DlBrandMark(modifier: Modifier = Modifier) {
    Box(
        modifier
            .size(36.dp)
            .clip(RoundedCornerShape(10.dp))
            .background(DlGreen),
        contentAlignment = Alignment.Center,
    ) {
        Text("灶", color = Color.White, fontWeight = FontWeight.Bold, fontSize = 15.sp)
    }
}

@Composable
fun DlBrandHeader(
    title: String,
    subtitle: String,
    modifier: Modifier = Modifier,
    trailing: @Composable (() -> Unit)? = null,
) {
    Row(
        modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        DlBrandMark()
        Column(Modifier.padding(start = 10.dp).weight(1f)) {
            Text(title, fontSize = 18.sp, fontWeight = FontWeight.Bold, color = DlInk, lineHeight = 20.sp)
            Text(subtitle, fontSize = 12.sp, color = DlMuted, modifier = Modifier.padding(top = 2.dp))
        }
        trailing?.invoke()
    }
}

/** 食堂范围 pill，对齐 mock `.picker`。 */
@Composable
fun DlCanteenPill(text: String, onClick: (() -> Unit)? = null, modifier: Modifier = Modifier) {
    val shape = RoundedCornerShape(999.dp)
    Row(
        modifier
            .clip(shape)
            .background(DlGreenSoft)
            .then(if (onClick != null) Modifier.clickable(onClick = onClick) else Modifier)
            .padding(horizontal = 10.dp, vertical = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        Text(text, color = DlGreen, fontSize = 12.sp, fontWeight = FontWeight.SemiBold)
        if (onClick != null) Text("▾", color = DlGreen, fontSize = 10.sp)
    }
}

/** 底栏选中态图标底，对齐 mock `.nav a.active .ico`。 */
@Composable
fun DlNavEmoji(emoji: String, selected: Boolean) {
    Box(
        Modifier
            .size(26.dp)
            .clip(RoundedCornerShape(8.dp))
            .background(if (selected) DlGreenSoft else Color.Transparent),
        contentAlignment = Alignment.Center,
    ) {
        Text(emoji, fontSize = 14.sp)
    }
}

/** 「更多」分组标题 + 白底设置行，对齐 Web / mock。 */
@Composable
fun DlSettingsGroup(title: String, content: @Composable () -> Unit) {
    Column(Modifier.fillMaxWidth().padding(bottom = 12.dp)) {
        Text(
            title,
            fontSize = 12.sp,
            color = DlMuted,
            fontWeight = FontWeight.SemiBold,
            modifier = Modifier.padding(horizontal = 6.dp, vertical = 4.dp),
        )
        Surface(
            shape = RoundedCornerShape(14.dp),
            color = DlCard,
            border = androidx.compose.foundation.BorderStroke(1.dp, DlLine),
            modifier = Modifier.fillMaxWidth(),
        ) {
            Column { content() }
        }
    }
}

@Composable
fun DlSettingsRow(
    label: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    testTag: String = "",
    danger: Boolean = false,
    trailing: @Composable (() -> Unit)? = null,
    showDivider: Boolean = true,
) {
    Column {
        Row(
            modifier
                .fillMaxWidth()
                .then(if (testTag.isNotEmpty()) Modifier.testTag(testTag) else Modifier)
                .clickable(onClick = onClick)
                .padding(horizontal = 14.dp, vertical = 14.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.SpaceBetween,
        ) {
            Text(label, fontSize = 15.sp, color = if (danger) DlDanger else DlInk)
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                trailing?.invoke()
                Text("›", color = DlGray, fontSize = 16.sp)
            }
        }
        if (showDivider) {
            Box(Modifier.fillMaxWidth().height(1.dp).background(DlLine))
        }
    }
}

@Composable
fun DlAccountChip(username: String, modifier: Modifier = Modifier) {
    val letter = username.trim().take(1).ifEmpty { "账" }
    Row(
        modifier
            .clip(RoundedCornerShape(999.dp))
            .border(1.dp, DlLine, RoundedCornerShape(999.dp))
            .background(Color(0xFFFAFAFA))
            .padding(start = 5.dp, end = 10.dp, top = 5.dp, bottom = 5.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Box(
            Modifier.size(26.dp).clip(CircleShape).background(DlGreenSoft),
            contentAlignment = Alignment.Center,
        ) {
            Text(letter, color = DlGreen, fontWeight = FontWeight.Bold, fontSize = 12.sp)
        }
        Text(username.ifBlank { "账号" }, fontSize = 13.sp, color = Color(0xFF374151))
    }
}

@Composable
fun DlMethodRow(
    title: String,
    subtitle: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    testTag: String = "",
) {
    Row(
        modifier
            .fillMaxWidth()
            .then(if (testTag.isNotEmpty()) Modifier.testTag(testTag) else Modifier)
            .clip(RoundedCornerShape(10.dp))
            .border(1.dp, DlLine, RoundedCornerShape(10.dp))
            .clickable(onClick = onClick)
            .padding(12.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.SpaceBetween,
    ) {
        Column(Modifier.weight(1f)) {
            Text(title, fontWeight = FontWeight.SemiBold, fontSize = 14.sp, color = DlInk)
            Text(subtitle, fontSize = 12.sp, color = DlGray, modifier = Modifier.padding(top = 2.dp))
        }
        Text("›", color = DlGray)
    }
}
