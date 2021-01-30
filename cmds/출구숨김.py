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
        hidden = 0
        for ex in exits:
            x = ex.split(None, 1)

            #if x[0][:len(x[0])-1] == line and x[0][-1] == '$': 
            if x[0][:-1] == line and x[0][-1] == '$': 
                #exit = x[0][:len(x[0])-1]
                exit = x[0][:-1]
                hidden = 1
            elif x[0] == line:
                exit = x[0] + '$'
                hidden = 2
            else:
                exit = x[0]

            nexit = exit + ' ' + x[1]

            if nexists == '':
                nexists = nexit
            else: 
                nexists = nexists + '\r\n' + nexit
            
        ob.env['출구'] = nexists
        del nexists
        ob.env.save()
        ob.env.init()

        if hidden == 2:
            ob.sendLine('☞ 출구가 숨겨졌습니다.')
        elif hidden == 1:
            ob.sendLine('☞ 출구가 드러났습니다.')
        else:
            ob.sendLine('☞ 그런 출구가 없습니다.')


