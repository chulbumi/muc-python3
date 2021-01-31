# -*- coding: utf-8 -*-

from objs.cmd import Command
from lib.hangul import *

class CmdObj(Command):

    def cmd(self, ob, line):
        if line == '':
            name = '당신'
            target = ob
        else:
            target = ob.env.findObjName(line)
            if target == None or is_player(target) == False:
                ob.sendLine('☞ 당신의 안광으로는 그런것을 볼수 없다네')
                return
            name = target['이름']
        c = 0
        tmp = ''
        for obj in target.objs:
            if obj['종류'] == '호위':
                try:
                    a = obj.hp
                except:
                    obj.hp = obj['체력']
                guard = obj
                c += 1
                hp = (obj.hp * 100 )// getItem(obj.index)['체력']
                
                tmp += '[1;32m·[0;36m%2d.%s[0;37m ː %s (%d)\r\n' % (c, obj['이름'], ob.strBar[hp//10] , hp)
        
        if c == 0:
            if target == ob:
                ob.sendLine('당신은 호위를 거느리지 않고 있습니다.')
            else:
                ob.sendLine('%s 호위를 거느리지 않고 있습니다.' % target.han_un())
            return
        msg = '━━━━━━━━━━━━━━━━━━━━━━━━━━━━\r\n'
        buf = '☞ %s의 호위 : %s, 호위수 : %d, 분노 : %d' % (name, guard['이름'], c, getInt(target['분노']))
        msg += '[1;44m%-56s[0;40m\r\n' % buf
        msg += '────────────────────────────\r\n'

        desc = guard['설명2']

        if type(desc) == list:
            msg += '\r\n'.join(desc) + '\r\n'
        else:
            msg += desc + '\r\n'

        msg += '────────────────────────────\r\n'
        msg += tmp
        msg += '━━━━━━━━━━━━━━━━━━━━━━━━━━━━'
        ob.sendLine(msg)


