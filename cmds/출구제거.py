# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):

        if getInt(ob['관리자등급']) < 1000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        if line == '':
            ob.sendLine('☞ 사용법: [출구] 출구숨김')
            return

        exits = ob.env['출구']
        nexists = ''
        c = 0
        for ex in exits:
            x = ex.split(None, 1)
            if x[0][:-1] == line and x[0][-1] == '$': 
                c = 1
                continue
            elif x[0] == line:
                c = 1
                continue
            nexit = x[0] + ' ' + x[1]
            if nexists == '':
                nexists = nexit
            else: 
                nexists = nexists + '\r\n' + nexit
            
        ob.env['출구'] = nexists
        del nexists
        ob.env.init()

        if c == 1:
            ob.sendLine('☞ 출구가 제거되었습니다.')
        else:
            ob.sendLine('☞ 그런 출구가 없습니다.')


