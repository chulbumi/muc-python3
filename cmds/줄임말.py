# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):
        if len(line) == 0:
            if len(ob.alias) == 0:
                ob.sendLine('☞ 줄임말이 설정되어 있지 않아요. ^^')
                return
            ob.sendLine('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━')
            ob.sendLine('[47m[30m◁ 줄임말 ▷                                                                  [40m[37m')
            ob.sendLine('───────────────────────────────────────')
            msg = ''
            for key in ob.alias:
                msg += '[%s] %s\r\n' % (key, ob.alias[key])
            ob.write(msg)
            ob.sendLine('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━')
            return

        wlist = line.split(None, 1)
        key = wlist[0]
        #인수가 하나면 줄임말 삭제
        if len(wlist) == 1:
            if ob.delAlias(key):
                ob.sendLine('☞ 줄임말을 제거하였어요. ^^')
            return
        
        data = wlist[1].strip()

        wlist = data.split(';')
        if key in wlist:
            ob.sendLine('☞ 중첩된 줄임말은 사용할 수 없어요. ^^')
            return
        for word in wlist:
            if word in ob.alias:
                ob.sendLine('☞ 중첩된 줄임말은 사용할 수 없어요. ^^')
                return
        if len(ob.alias) >= 100:
            ob.sendLine('☞ 줄임말이 너무 많아요. ^^')
            return
            
        
        if ob.setAlias(key, data):
            ob.sendLine('☞ 줄임말을 설정하였어요. ^^')
        

