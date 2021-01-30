# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):
        if ob['직위'] != '방주':
            ob.sendLine('☞ 방파의 방주만이 할 수 있습니다.')
            return
        if line == '':
            ob.sendLine('☞ 사용법 : [대상] 방주권한양도')
            return

        obj = ob.env.findObjName(line)
        if obj == None or is_player(obj) == False:
            ob.sendLine('☞ 이곳에 그런 무림인이 없습니다.')
            return
        if obj == ob:
            ob.sendLine('☞ 이미 당신은 방주 입니다.')
            return
        if obj['소속'] != ob['소속']:
            ob.sendLine('☞ 당신의 소속이 아닙니다.')
            return
        if obj['직위'] != '부방주':
            ob.sendLine('☞ 방주권한은 부방주에게만 양도할 수 있습니다.')
            return
        if MAIN_CONFIG['부방주양도레벨'] > obj['레벨']:
            ob.sendLine('☞ 방주가 되기에는 역량이 부족합니다.')
            return
        
        obj['직위'] = '방주'
        ob['직위'] = '부방주'
        g = GUILD[ob['소속']]
        g['부방주리스트'].append(ob['이름'])
        g['부방주리스트'].remove(obj['이름'])
        g['방주이름'] = obj['이름']
        GUILD.save()

        msg = '%s %s 방주로 권한이양을 선포합니다.' % (ob.han_iga(), obj.han_obj())
#obj.sendLine('\r\n당신은 파문되었습니다.')
        obj.lpPrompt()
        ob.sendGroup(msg, prompt = True)
        
