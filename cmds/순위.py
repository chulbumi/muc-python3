# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    level = 1000
    def cmd(self, ob, line):
        #if getInt(ob['관리자등급']) < 1000:
        #    ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
        #    return
        if line == '':
            ob.sendLine('☞ 사용법: [특성치] 순위')
            return
        
        if getInt(ob['관리자등급']) < 1000:
            if line not in ['힘', '은전', '레벨', '체력', '내공', '민첩', '맷집', '명중', '회피', '필살', '운', '나이']:
                ob.sendLine('☞ 사용법: [특성치] 순위')
                return
        if line == '내공':
            line = '최고내공'
        if line == '체력':
            line = '최고체력'
        if line == '민첩':
            line = '민첩성'

        if ob['은전'] < 100000:
            ob.sendLine('☞ 은전이 부족해요.')
            return

        ob['은전'] = ob['은전'] - 100000
        l = []
        for c in ob.channel.players:
            if c['이름'] != '':
                if getInt(c['관리자등급']) != 0:
                    continue
                v = c[line]
                if v == '' or v == 0:
                    continue
                l.append((c['이름'], v))
            
        l.sort(reverse=True,key=lambda tup: tup[1])
        msg = ''
        cnt = 0
        for c in l:
            cnt += 1
            if getInt(ob['관리자등급']) >= 1000:
                msg += '%10s %-13d  ' % (c[0], c[1])
                if cnt % 3 == 0:
                    msg += '\r\n'
            else:
                msg += '[%02d] %-10s ' % (cnt, c[0]) 
                if cnt % 5 == 0:
                    msg += '\r\n'

            if cnt == 30:
                break
        ob.sendLine(msg)

