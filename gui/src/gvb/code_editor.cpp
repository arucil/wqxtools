#include "code_editor.h"

#include <QToolTip>

CodeEditor::CodeEditor(QWidget *parent) :
  ScintillaEdit(parent),
  m_dirty(false) {
  connect(this, &ScintillaEdit::notify, this, &CodeEditor::notified);
}

void CodeEditor::notified(Scintilla::NotificationData *data) {
  switch (data->nmhdr.code) {
    case Scintilla::Notification::SavePointReached:
      if (m_dirty != false) {
        emit dirtyChanged(false);
      }
      m_dirty = false;
      break;
    case Scintilla::Notification::SavePointLeft:
      if (m_dirty != true) {
        emit dirtyChanged(true);
      }
      m_dirty = true;
      break;
    case Scintilla::Notification::Modified: {
      if (!m_textLoaded) {
        return;
      }

      auto bits = static_cast<int>(data->modificationType);

      if (!m_timerModify) {
        QTimer::singleShot(500, this, &GvbEditor::modified);
        m_timerModify = true;
      }

      m_actUndo->setEnabled(m_edit->canUndo());
      m_actRedo->setEnabled(m_edit->canRedo());
      if (bits & SC_MOD_INSERTTEXT) {
        InsertText *insert;
        if (
          !m_edits.empty() && (insert = get_if<InsertText>(&m_edits.back()))
          && insert->pos + insert->str.size()
            == static_cast<size_t>(data->position)) {
          insert->str.append(data->text, data->length);
        } else {
          InsertText insert = {
            static_cast<size_t>(data->position),
            std::string(data->text, data->length)};
          m_edits.push_back(insert);
        }
      } else if (bits & SC_MOD_DELETETEXT) {
        DeleteText *del;
        if (
          !m_edits.empty() && (del = get_if<DeleteText>(&m_edits.back()))
          && del->pos == static_cast<size_t>(data->position + data->length)) {
          del->len += static_cast<size_t>(data->length);
          del->pos -= static_cast<size_t>(data->length);
        } else {
          DeleteText del = {
            static_cast<size_t>(data->position),
            static_cast<size_t>(data->length)};
          m_edits.push_back(del);
        }
      }
      break;
    }
    case Scintilla::Notification::DwellStart: {
      if (data->position < 0 || data->position > m_edit->length()) {
        break;
      }
      auto pos = static_cast<size_t>(data->position);
      std::string messages;
      m_diagRanges.overlap_find_all({pos, pos}, [&messages, this](auto it) {
        if (!messages.empty()) {
          messages += '\n';
        }
        messages += "▸ ";
        messages += m_diagnostics[it->interval().index].message.c_str();
        return true;
      });
      if (!messages.empty()) {
        m_edit->callTipShow(data->position, messages.c_str());
      }
      break;
    }
    case Scintilla::Notification::DwellEnd:
      QToolTip::hideText();
      break;
    case Scintilla::Notification::UpdateUI:
      if (
        static_cast<int>(data->updated)
        & (SC_UPDATE_SELECTION | SC_UPDATE_CONTENT)) {
        emit cursorPositionChanged(currentPos());
      }
      break;
    default:
      break;
  }
}