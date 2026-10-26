package com.duolinzao.app.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Card
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.duolinzao.app.api.Canteen
import com.duolinzao.app.api.Session
import com.duolinzao.app.api.addDays
import com.duolinzao.app.api.shanghaiToday
import kotlinx.coroutines.launch

/**
 * 食堂竖屏壳（C1）：底栏四 Tab（人数 | 选菜 | 菜谱 | 更多）。
 * 人数页接真接口；选菜/菜谱占位到 C2/C3；更多含账户/切换员工/退出。
 */
@Composable
fun CanteenShellScreen(onLogout: () -> Unit, onSwitchEmployee: () -> Unit) {
    var tab by remember { mutableStateOf(0) }
    Scaffold(
        bottomBar = {
            NavigationBar {
                NavigationBarItem(selected = tab == 0, onClick = { tab = 0 }, icon = {}, label = { Text("人数") }, modifier = Modifier.testTag("tab_headcount"))
                NavigationBarItem(selected = tab == 1, onClick = { tab = 1 }, icon = {}, label = { Text("选菜") }, modifier = Modifier.testTag("tab_optimize"))
                NavigationBarItem(selected = tab == 2, onClick = { tab = 2 }, icon = {}, label = { Text("菜谱") }, modifier = Modifier.testTag("tab_menu"))
                NavigationBarItem(selected = tab == 3, onClick = { tab = 3 }, icon = {}, label = { Text("更多") }, modifier = Modifier.testTag("tab_more"))
            }
        },
    ) { padding ->
        Column(Modifier.padding(padding).fillMaxSize()) {
            CanteenTopBar()
            when (tab) {
                0 -> HeadcountTab()
                1 -> PlaceholderTab("选菜", "周视图与当餐勾选（C3 接入，与 Web 选菜优化同接口）。")
                2 -> PlaceholderTab("菜谱", "菜/饭/汤分组与新建编辑（C2 接入，与 Web 菜谱同接口）。")
                3 -> MoreTab(onLogout = onLogout, onSwitchEmployee = onSwitchEmployee)
            }
        }
    }
}

/** 顶栏：管理范围内食堂选择器；仅 1 个时只读展示。 */
@Composable
private fun CanteenTopBar() {
    val me = Session.me.collectAsState().value
    val scope = rememberCoroutineScope()
    var canteens by remember { mutableStateOf<List<Canteen>>(emptyList()) }
    var picked by remember { mutableStateOf("") }
    LaunchedEffect(me) {
        scope.launch {
            try {
                val all = Session.repo.allCanteens()
                val ids = me?.roles?.flatMap { it.canteen_ids }?.toSet()
                canteens = if (me?.roles?.any { it.role == "super_admin" } == true || ids.isNullOrEmpty()) all
                    else all.filter { ids.contains(it.id) }
                if (picked.isEmpty()) picked = canteens.firstOrNull()?.id.orEmpty()
            } catch (_: Exception) { }
        }
    }
    Row(
        Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text("多邻灶 · 食堂工作台", style = MaterialTheme.typography.titleMedium, modifier = Modifier.weight(1f))
        if (canteens.size <= 1) {
            Text(canteens.firstOrNull()?.name.orEmpty(), color = MaterialTheme.colorScheme.primary)
        } else {
            canteens.forEach {
                TextButton(onClick = { picked = it.id }, modifier = Modifier.testTag("canteen_pick_${it.id}")) {
                    Text(if (it.id == picked) "● ${it.name}" else it.name)
                }
            }
        }
    }
}

@Composable
private fun HeadcountTab() {
    val scope = rememberCoroutineScope()
    var date by remember { mutableStateOf(shanghaiToday()) }
    var counts by remember { mutableStateOf(mapOf("breakfast" to 0, "lunch" to 0, "dinner" to 0)) }
    var locked by remember { mutableStateOf(false) }
    var canteenId by remember { mutableStateOf("") }
    var error by remember { mutableStateOf<String?>(null) }
    LaunchedEffect(Unit) {
        scope.launch {
            try {
                val all = Session.repo.allCanteens()
                canteenId = all.firstOrNull()?.id.orEmpty()
            } catch (_: Exception) { }
        }
    }
    LaunchedEffect(date, canteenId) {
        if (canteenId.isEmpty()) return@LaunchedEffect
        scope.launch {
            try {
                val hc = Session.repo.headcount(canteenId, date)
                counts = mapOf(
                    "breakfast" to (hc.slots["breakfast"] ?: 0),
                    "lunch" to (hc.slots["lunch"] ?: 0),
                    "dinner" to (hc.slots["dinner"] ?: 0),
                )
                locked = hc.locked
                error = null
            } catch (e: Exception) {
                error = e.message ?: "加载失败"
            }
        }
    }
    Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
            TextButton(onClick = { date = addDays(date, -1) }) { Text("‹") }
            Text(date, modifier = Modifier.testTag("headcount_date"))
            TextButton(onClick = { date = addDays(date, 1) }) { Text("›") }
        }
        if (error != null) Text(error!!, color = MaterialTheme.colorScheme.error)
        listOf("breakfast" to "早餐", "lunch" to "午餐", "dinner" to "晚餐").forEach { (slot, name) ->
            Card(Modifier.fillMaxWidth().testTag("headcount_card_$slot")) {
                Row(Modifier.fillMaxWidth().padding(18.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.SpaceBetween) {
                    Column {
                        Text(name, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        Text("有效报餐", style = MaterialTheme.typography.titleMedium)
                    }
                    Column(horizontalAlignment = Alignment.End) {
                        Text("${counts[slot] ?: 0}", fontSize = 40.sp, color = MaterialTheme.colorScheme.primary)
                        Text("人", color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                }
            }
        }
        Text(if (locked) "该日三餐已锁定（一次锁定）" else "该日三餐尚未锁定", color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

@Composable
private fun PlaceholderTab(title: String, body: String) {
    Column(Modifier.fillMaxSize().padding(24.dp), verticalArrangement = Arrangement.Center, horizontalAlignment = Alignment.CenterHorizontally) {
        Text(title, style = MaterialTheme.typography.titleLarge)
        Text(body, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.padding(top = 8.dp))
    }
}

@Composable
private fun MoreTab(onLogout: () -> Unit, onSwitchEmployee: () -> Unit) {
    val me = Session.me.collectAsState().value
    val scope = rememberCoroutineScope()
    val canEmployee = me?.permissions?.any { it == "meal.self_read" || it == "shift.self_write" } == true
    Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text("账户：${me?.personName().orEmpty()}", style = MaterialTheme.typography.titleMedium)
        Text("强提醒 / 评分汇总 / 评语 / 改单 / 邀请码：Web 横屏已支持，App 后续迭代接入。", color = MaterialTheme.colorScheme.onSurfaceVariant)
        if (canEmployee) {
            TextButton(onClick = onSwitchEmployee, modifier = Modifier.testTag("canteen_to_employee")) { Text("切换到员工端") }
        }
        TextButton(onClick = {
            scope.launch {
                try { Session.repo.logout() } catch (_: Exception) { }
                Session.logoutLocal()
                onLogout()
            }
        }, modifier = Modifier.testTag("canteen_logout")) { Text("退出登录") }
    }
}
