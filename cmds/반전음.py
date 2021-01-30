# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):
        if line == '':
            ob.sendLine('☞ 사용법: [내용] 반전음(:)')
            return
        words = line.split()
        if ob._talker == None:
            ob.sendLine('☞ 전음이 전달될만한 상대가 없어요. ^^')
            return
        if ob._talker not in ob.channel.players:
            ob._talker = None
            ob.sendLine('☞ 전음이 전달될만한 상대가 없어요. ^^')
            return
        ply = ob._talker

        if ob.checkConfig('전음거부') or ply.checkConfig('전음거부'):
            ob.sendLine('☞ 전음 거부중이에요. ^^')
            return
        if ob.env.noComm():
            ob.sendLine('☞ 이지역에서는 어떠한 통신도 불가능합니다.')
            return
        msg = ''
        for i in range(0, len(words)):
            msg += words[i] + ' ' 
        msg1 = '[[1m[36m전음[0m[37m] %s에게 보냄 : %s' % (ply['이름'], msg)
        msg2 = '[[1m[36m전음[0m[37m] %s : %s' % (ob['이름'], msg)

        ob.sendLine(msg1)
        ply._talker = ob
        ply.sendLine('\r\n' + msg2)
        ply.talkHistory.append(msg2)
        if len(ply.talkHistory) > 22:
            ply.talkHistory.__delitem__(0)
        ply.lpPrompt()
