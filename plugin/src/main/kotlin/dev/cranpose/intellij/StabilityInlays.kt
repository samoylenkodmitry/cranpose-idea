package dev.cranpose.intellij

import com.intellij.openapi.editor.Editor
import com.intellij.openapi.editor.EditorCustomElementRenderer
import com.intellij.openapi.editor.Inlay
import com.intellij.openapi.editor.colors.EditorFontType
import com.intellij.openapi.editor.markup.TextAttributes
import com.intellij.ui.JBColor
import java.awt.Color
import java.awt.Font
import java.awt.Graphics
import java.awt.Graphics2D
import java.awt.Rectangle
import java.awt.RenderingHints

/** The editor supplies placement, font scale and theme; Rust supplies the badge content. */
class StabilityBadgeRenderer(val badge: StabilityBadge) : EditorCustomElementRenderer {
    private fun font(editor: Editor): Font = editor.colorsScheme.getFont(EditorFontType.PLAIN)
        .deriveFont((editor.colorsScheme.editorFontSize * 0.82f).coerceAtLeast(9f))
    private fun foreground(): Color = when (badge.tone) {
        "stable" -> JBColor(Color(0x3F7058), Color(0x9CC8B2))
        "warning" -> JBColor(Color(0x825D13), Color(0xD9BA75))
        "danger" -> JBColor(Color(0xA24442), Color(0xE4A19B))
        else -> JBColor(Color(0x6A6E78), Color(0x969DAA))
    }
    private fun background(): Color = when (badge.tone) {
        "stable" -> JBColor(Color(0xEBF3EE), Color(0x28372F))
        "warning" -> JBColor(Color(0xF8F0DC), Color(0x3B3426))
        "danger" -> JBColor(Color(0xFAEAE8), Color(0x402C2C))
        else -> JBColor(Color(0xEFF0F2), Color(0x303238))
    }
    override fun calcWidthInPixels(inlay: Inlay<*>): Int =
        inlay.editor.contentComponent.getFontMetrics(font(inlay.editor)).stringWidth(badge.label) + 16
    override fun paint(inlay: Inlay<*>, graphics: Graphics, targetRegion: Rectangle, textAttributes: TextAttributes) {
        val g = graphics.create() as Graphics2D
        try {
            g.setRenderingHint(RenderingHints.KEY_ANTIALIASING, RenderingHints.VALUE_ANTIALIAS_ON)
            g.font = font(inlay.editor)
            val metrics = g.fontMetrics
            val height = (metrics.height + 2).coerceAtMost(targetRegion.height)
            val top = targetRegion.y + (targetRegion.height - height) / 2
            g.color = background()
            g.fillRoundRect(targetRegion.x + 3, top, targetRegion.width - 6, height, 6, 6)
            g.color = foreground()
            g.drawString(badge.label, targetRegion.x + 8, top + (height - metrics.height) / 2 + metrics.ascent)
        } finally { g.dispose() }
    }
}

object StabilityInlays {
    fun replace(editor: Editor, badges: List<StabilityBadge>, showStable: Boolean) {
        editor.inlayModel.execute(true) {
            clear(editor)
            for (badge in badges) {
                if (!showStable && badge.tone == "stable") continue
                if (badge.start < 0 || badge.end < badge.start || badge.end > editor.document.textLength) continue
                editor.inlayModel.addInlineElement(badge.end, true, StabilityBadgeRenderer(badge))
            }
        }
    }
    fun clear(editor: Editor) {
        editor.inlayModel.getInlineElementsInRange(0, editor.document.textLength, StabilityBadgeRenderer::class.java)
            .forEach { it.dispose() }
    }
}
